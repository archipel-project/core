mod chunk_mesh;
mod ordered_chunk_pos;
mod texture_atlas;

use super::camera::{Camera, CameraFrustum};
use super::{Context, RenderJob};
use crate::graphic::terrain::chunk_mesh::ChunkMesh;
use crate::graphic::terrain::ordered_chunk_pos::OrderedChunkPos;
use crate::graphic::terrain::texture_atlas::{TextureAtlas, TextureAtlasBuilder};
use std::collections::BTreeMap;
use utils::spare_set::{Id, SparseSet};
use wgpu::util::DeviceExt;
use world_core::{Chunk, ChunkManager};

pub struct TerrainRenderer {
    render_pipeline: wgpu::RenderPipeline,
    texture_atlas: TextureAtlas,
    chunks_meshes: BTreeMap<OrderedChunkPos, ChunkMesh>,
    cache: MeshCache,
    render_distance: i32,
    last_frustum: CameraFrustum,
}

impl TerrainRenderer {
    pub fn new(
        camera: &Camera,
        render_distance: i32,
        chunk_manager: &ChunkManager,
        context: &Context,
    ) -> Self {
        //todo: change that to a proper resource manager

        let load_texture = |buffer: &[u8]| image::load_from_memory(buffer).unwrap().to_rgba8();

        let builder = TextureAtlasBuilder {
            vec: vec![
                load_texture(include_bytes!("textures/stone.png")),
                load_texture(include_bytes!("textures/diamond_block.png")),
                load_texture(include_bytes!("textures/emerald_block.png")),
                load_texture(include_bytes!("textures/lapis_block.png")),
                load_texture(include_bytes!("textures/gold_block.png")),
                load_texture(include_bytes!("textures/iron_block.png")),
                load_texture(include_bytes!("textures/coal_block.png")),
                load_texture(include_bytes!("textures/wool_colored_red.png")),
                load_texture(include_bytes!("textures/hay_block_top.png")),
                load_texture(include_bytes!("textures/hay_block_side.png")),
                load_texture(include_bytes!("textures/grass_block_top.png")),
            ],
        };

        let texture_atlas = TextureAtlas::new_exp(builder, 16, context);

        let shader = context
            .wgpu_device
            .create_shader_module(wgpu::include_wgsl!("terrain.wgsl"));
        let render_pipeline_layout =
            context
                .wgpu_device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Terrain Render Pipeline Layout"),
                    bind_group_layouts: &[
                        camera.get_bind_group_layout(),        //0
                        texture_atlas.get_bind_group_layout(), //1
                    ],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            context
                .wgpu_device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                });

        let mut chunks_meshes = BTreeMap::new();
        let frustum = camera.get_frustum(render_distance);
        let chunks_to_display = chunk_manager
            .get_chunk_with_predicate(frustum.get_aabb(), |aabb| frustum.contains(&aabb));
        for chunk in chunks_to_display {
            if let Some(mesh) =
                ChunkMesh::build_from(chunk_manager, chunk.position(), &texture_atlas, context)
            {
                chunks_meshes.insert(chunk.position().into(), mesh);
            }
        }

        let cache_size = (render_distance as usize * 2).pow(3);

        Self {
            render_distance,
            render_pipeline,
            texture_atlas,
            chunks_meshes,
            last_frustum: frustum,
            cache: MeshCache::new(cache_size),
        }
    }

    pub fn rendered_mesh_count(&self) -> usize {
        self.chunks_meshes.len()
    }

    pub fn build_render_job<'a>(
        &'a mut self,
        chunk_manager: &'a mut ChunkManager,
        camera: &'a Camera,
        context: &'a Context,
    ) -> TerrainRenderJob<'a> {
        let old_frustum = &self.last_frustum;
        let new_frustum = camera.get_frustum(self.render_distance);

        //difference between two frustum
        let frustum_diff = |aabb, frustum1: &CameraFrustum, frustum2: &CameraFrustum| {
            frustum1.contains(&aabb)
                && if aabb.is_unit() {
                    !(frustum2.contains(&aabb) && frustum2.get_aabb().intersects(&aabb))
                } else {
                    true
                }
        };

        //add new visible chunks
        {
            let add_chunk = |id, chunk: &Chunk| {
                let mesh = self.cache.get_mesh(id).unwrap_or_else(|| {
                    ChunkMesh::build_from(
                        chunk_manager,
                        chunk.position(),
                        &self.texture_atlas,
                        context,
                    )
                });
                if let Some(mesh) = mesh {
                    self.chunks_meshes.insert(chunk.position().into(), mesh);
                }
            };
            chunk_manager.foreach_chunk_with_predicate(
                new_frustum.get_aabb(),
                |aabb| frustum_diff(aabb, &new_frustum, old_frustum),
                add_chunk,
            );
        }

        //remove old visible chunks
        {
            let remove_chunk = |id, chunk: &Chunk| {
                let mesh = self.chunks_meshes.remove(&chunk.position().into());
                self.cache.add_mesh(id, mesh);
            };
            chunk_manager.foreach_chunk_with_predicate(
                old_frustum.get_aabb(),
                |aabb| frustum_diff(aabb, old_frustum, &new_frustum),
                remove_chunk,
            );
        }

        self.last_frustum = new_frustum;

        let pos = self
            .chunks_meshes
            .keys()
            .map(|pos| {
                let pos = pos.0;
                ChunkPosAttribute {
                    position: [pos.x, pos.y, pos.z],
                }
            })
            .collect::<Vec<_>>();

        let pos_buffer =
            context
                .wgpu_device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

        for (chunk_index, (_pos, chunk_mesh)) in
            self.terrain_renderer.chunks_meshes.iter().enumerate()
        {
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

struct MeshCache {
    cached_meshes: SparseSet<(u16, Option<ChunkMesh>)>,
    size: usize,
    date: u16,
    oldest: u16,
}

impl MeshCache {
    fn new(size: usize) -> Self {
        Self {
            cached_meshes: SparseSet::with_capacity(size),
            size,
            date: 0,
            oldest: 0,
        }
    }

    ///get the mesh from the cache and remove if it exists
    fn get_mesh(&mut self, chunk_id: Id) -> Option<Option<ChunkMesh>> {
        self.cached_meshes.remove(chunk_id).map(|(_, mesh)| mesh)
    }

    fn add_mesh(&mut self, chunk_id: Id, mesh: Option<ChunkMesh>) {
        if self.cached_meshes.len() >= self.size {
            self.remove_oldest_mesh();
        }

        self.cached_meshes.insert(chunk_id, (self.date, mesh));
        self.date = self.date.wrapping_add(1);
    }

    fn remove_oldest_mesh(&mut self) {
        let mut oldest_id = None;
        for (id, (date, _)) in self.cached_meshes.iter() {
            if *date == self.oldest {
                oldest_id = Some(id);
                break;
            }
        }
        self.cached_meshes.remove(oldest_id.unwrap());
        self.oldest = self.oldest.wrapping_add(1);
    }
}
