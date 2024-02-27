mod texture_atlas;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use wgpu::util::DeviceExt;
use math::AABB;
use world_core::{ChunkPos, ChunkManager, chunk, block_state::AIR};
use crate::graphic::terrain::texture_atlas::{TextureAtlas, TextureAtlasBuilder, TextureCoordinates};
use super::camera::Camera;
use super::{Context, RenderJob};



struct OrderedChunkPos(ChunkPos);

impl From<ChunkPos> for OrderedChunkPos {
    fn from(pos: ChunkPos) -> Self {
        Self(pos)
    }
}

impl Into<ChunkPos> for OrderedChunkPos {
    fn into(self) -> ChunkPos {
        self.0
    }
}

impl Eq for OrderedChunkPos {}

impl PartialEq<Self> for OrderedChunkPos {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd<Self> for OrderedChunkPos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let x = self.0.x.cmp(&other.0.x);
        if x != Ordering::Equal {
            return Some(x);
        }
        let y = self.0.y.cmp(&other.0.y);
        if y != Ordering::Equal {
            return Some(y);
        }
        Some(self.0.z.cmp(&other.0.z))
    }
}

impl Ord for OrderedChunkPos {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}



pub struct TerrainRenderer {
    render_pipeline: wgpu::RenderPipeline,
    texture_atlas: TextureAtlas,
    chunks_meshes: BTreeMap<OrderedChunkPos, ChunkMesh>,
    render_distance: i32,
    last_camera_pos: ChunkPos,
}

impl TerrainRenderer{

    pub fn new(camera: &Camera, render_distance: i32, chunk_manager: &ChunkManager, context: &Context) -> Self {

        //todo: change that to a proper resource manager
        let builder = TextureAtlasBuilder{
            vec: vec![ image::load_from_memory(include_bytes!("textures/diamond_block.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/emerald_block.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/lapis_block.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/gold_block.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/iron_block.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/coal_block.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/wool_colored_red.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/hay_block_top.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/hay_block_side.png")).unwrap().to_rgba8(),
                       image::load_from_memory(include_bytes!("textures/grass_block_top.png")).unwrap().to_rgba8(),
            ],
        };

        let texture_atlas = TextureAtlas::new_exp(builder, 16, context);

        let shader = context.wgpu_device.create_shader_module(wgpu::include_wgsl!("terrain.wgsl"));
        let render_pipeline_layout = context.wgpu_device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Terrain Render Pipeline Layout"),
                bind_group_layouts: &[
                    camera.get_bind_group_layout(), //0
                    texture_atlas.get_bind_group_layout(), //1
                ],
                push_constant_ranges: &[],
            }
        );

        let render_pipeline = context.wgpu_device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Terrain Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc(), ChunkPosAttribute::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: super::Window::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            }
        );

        let camera_pos = camera.position.as_ivec3().div_euclid([chunk::SIZE, chunk::SIZE, chunk::SIZE].into());
        let mut chunks_meshes = BTreeMap::new();

        let camera_aabb = AABB::new(camera_pos - ChunkPos::ONE * render_distance, camera_pos + ChunkPos::ONE * render_distance);
        let chunk_to_display = chunk_manager.get_chunks_in(camera_aabb);
        for chunk in chunk_to_display {
            if let Some(mesh) = ChunkMesh::build_from(chunk_manager, chunk.get_position(), &texture_atlas, context) {
                chunks_meshes.insert(chunk.get_position().into(), mesh);
            }
        }

        //todo: load the chunks around the camera

        Self {
            render_distance,
            render_pipeline,
            texture_atlas,
            chunks_meshes,
            last_camera_pos: camera_pos,
        }
    }

    pub fn build_render_job<'a>(&'a mut self, chunk_manager: &'a mut ChunkManager, camera: &'a Camera, context: &'a Context) -> TerrainRenderJob<'a> {

        let old_camera_pos = self.last_camera_pos;
        let old_camera_aabb = AABB::new(old_camera_pos - ChunkPos::ONE * self.render_distance, old_camera_pos + ChunkPos::ONE * self.render_distance);
        let camera_pos = camera.position.as_ivec3().div_euclid([chunk::SIZE, chunk::SIZE, chunk::SIZE].into());
        let camera_aabb = AABB::new(camera_pos - ChunkPos::ONE * self.render_distance, camera_pos + ChunkPos::ONE * self.render_distance);
        self.last_camera_pos = camera_pos; //update the last camera position for the next frame

        //remove all chunks that are not in the render distance
        let chunk_to_remove = chunk_manager.get_chunk_with_predicate(old_camera_aabb, |aabb| !camera_aabb.totally_contains(&aabb));
        for chunk in chunk_to_remove {
            self.chunks_meshes.remove(&chunk.get_position().into());
        }

        //get the new chunks to display
        let new_chunk_to_display = chunk_manager.get_chunk_with_predicate(camera_aabb, |aabb| !old_camera_aabb.totally_contains(&aabb));
        for chunk in new_chunk_to_display {
             if let Some(mesh) = ChunkMesh::build_from(chunk_manager, chunk.get_position(), &self.texture_atlas, context) {
                self.chunks_meshes.insert(chunk.get_position().into(), mesh);
             }
        }

        let pos = self.chunks_meshes.keys().map(|pos|{
            let pos = pos.0;
            ChunkPosAttribute {
                position: [pos.x, pos.y, pos.z],
            }
        } ).collect::<Vec<_>>();

        let pos_buffer = context.wgpu_device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Position Buffer"),
            contents: bytemuck::cast_slice(&pos),
            usage: wgpu::BufferUsages::VERTEX,
        });

        TerrainRenderJob {
            terrain_renderer: self,
            camera,
            pos_buffer,
        }
    }
}

pub struct TerrainRenderJob<'a> {
    terrain_renderer: &'a TerrainRenderer,
    camera: &'a Camera,
    pos_buffer: wgpu::Buffer,
}

impl RenderJob for TerrainRenderJob<'_> {
    fn update(&mut self, _command_encoder: &mut wgpu::CommandEncoder, _render_context: &Context) {
        //nothing to do for now
    }

    fn draw<'pass>(&'pass mut self, render_pass: &mut wgpu::RenderPass<'pass>) {
        let terrain_renderer = &self.terrain_renderer;
        render_pass.set_bind_group(0, &self.camera.get_bind_group(), &[]);
        render_pass.set_bind_group(1, terrain_renderer.texture_atlas.get_bind_group(), &[]);
        render_pass.set_pipeline(&self.terrain_renderer.render_pipeline);


        for (chunk_index, (_pos, chunk_mesh)) in self.terrain_renderer.chunks_meshes.iter().enumerate() {
            render_pass.set_vertex_buffer(1, self.pos_buffer.slice(..));
            chunk_mesh.draw(render_pass, chunk_index);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    texture_coords: [f32; 2],
    texture_index: u32,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkPosAttribute {
    position: [i32; 3],
}

impl ChunkPosAttribute {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![
        3 => Sint32x3,
    ];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ChunkPosAttribute>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct ChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl ChunkMesh{

    fn build_from(chunk_manager: &ChunkManager, pos: ChunkPos, texture_atlas: &TextureAtlas, context: &Context) -> Option<Self> {
        let chunk = chunk_manager.get_chunk(pos)?;
        if chunk.is_empty() {
            return None;
        }
        let top_chunk = chunk_manager.get_chunk(pos + ChunkPos::new(0, 1, 0));
        let bottom_chunk = chunk_manager.get_chunk(pos + ChunkPos::new(0, -1, 0));
        let west_chunk = chunk_manager.get_chunk(pos + ChunkPos::new(-1, 0, 0));
        let east_chunk = chunk_manager.get_chunk(pos + ChunkPos::new(1, 0, 0));
        let north_chunk = chunk_manager.get_chunk(pos + ChunkPos::new(0, 0, -1));
        let south_chunk = chunk_manager.get_chunk(pos + ChunkPos::new(0, 0, 1));


        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        enum Face {
            Top,
            Bottom,
            West, //x-
            East, //X+
            North, //z-
            South, //z+
        }

        let get_block_at = |x: i32, y: i32, z: i32| {
            if x >= 0 && x < chunk::SIZE
                && y >= 0 && y < chunk::SIZE
                && z >= 0 && z < chunk::SIZE {
                return chunk.get_block_at(x, y, z)
            }
            if x < 0 {
                return west_chunk.map_or(AIR, |c| c.get_block_at(x + chunk::SIZE, y, z));
            }
            if x >= chunk::SIZE {
                return east_chunk.map_or(AIR, |c| c.get_block_at(x - chunk::SIZE, y, z));
            }
            if y < 0 {
                return bottom_chunk.map_or(AIR, |c| c.get_block_at(x, y + chunk::SIZE, z));
            }
            if y >= chunk::SIZE {
                return top_chunk.map_or(AIR, |c| c.get_block_at(x, y - chunk::SIZE, z));
            }
            if z < 0 {
                return north_chunk.map_or(AIR, |c| c.get_block_at(x, y, z + chunk::SIZE));
            }
            if z >= chunk::SIZE {
                return south_chunk.map_or(AIR, |c| c.get_block_at(x, y, z - chunk::SIZE));
            }
            AIR
        };


        //no clue why but if (0, 0, 0) is the first corner of the block in minecraft
        //then the second one is at (1, 1, -1), why the z is negative is beyond me
        let mut add_face = |x, y, z, face: Face, texture: TextureCoordinates, texture_index: u32| {
            match face {
                Face::Top => {
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z - 1.0],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index,

                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index
                    });
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 4);

                    indices.push(vertices.len() as u32 - 1);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 4);
                }
                Face::Bottom => {
                    vertices.push(Vertex {
                        position: [x, y, z - 1.0],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index
                    });
                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 2);

                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 1);
                }
                Face::West => {
                    vertices.push(Vertex {
                        position: [x, y, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z - 1.0],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y, z],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index
                    });
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 4);

                    indices.push(vertices.len() as u32 - 1);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 4);
                }
                Face::East => {
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z - 1.0],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z - 1.0],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index
                    });
                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 2);

                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 1);
                }
                Face::North => {
                    vertices.push(Vertex {
                        position: [x, y, z - 1.0],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z - 1.0],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z - 1.0],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index
                    });
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 4);

                    indices.push(vertices.len() as u32 - 1);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 4);
                }
                Face::South => {
                    vertices.push(Vertex {
                        position: [x, y, z],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index
                    });
                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 2);

                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 1);
                }
            }
        };

        for y in 0..chunk::SIZE {
            for z in 0..chunk::SIZE  {
                for x in 0..chunk::SIZE {
                    let blockstate = chunk.get_block_at(x, y, z);
                    if blockstate == AIR { continue; }

                    let texture_coordinates = texture_atlas.get_texture_coordinates();
                    let fx = x as f32;
                    let fy = y as f32;
                    let fz = z as f32;
                    if get_block_at(x, y + 1, z) == AIR {
                        add_face(fx, fy, fz, Face::Top, texture_coordinates, blockstate as u32);
                    }
                    if get_block_at(x, y - 1, z) == AIR {
                        add_face(fx, fy, fz, Face::Bottom, texture_coordinates, blockstate as u32);
                    }
                    if get_block_at(x - 1, y, z) == AIR {
                        add_face(fx, fy, fz, Face::West, texture_coordinates, blockstate as u32);
                    }
                    if get_block_at(x + 1, y, z) == AIR {
                        add_face(fx, fy, fz, Face::East, texture_coordinates, blockstate as u32);
                    }
                    if get_block_at(x, y, z - 1) == AIR {
                        add_face(fx, fy, fz, Face::North, texture_coordinates, blockstate as u32);
                    }
                    if get_block_at(x, y, z + 1) == AIR {
                        add_face(fx, fy, fz, Face::South, texture_coordinates, blockstate as u32);
                    }
                }
            }
        }

        if vertices.is_empty() && indices.is_empty() {
            return None;
        }

        Some(Self::new(&context.wgpu_device, &vertices, &indices))
    }

    fn new(device: &wgpu::Device, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        let index_count = indices.len() as u32;
        Self {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }

    fn draw<'pass>(&'pass self, render_pass: &mut wgpu::RenderPass<'pass>, pos_index: usize) {
        let pos_index = pos_index as u32;
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, pos_index..pos_index + 1);
    }
}