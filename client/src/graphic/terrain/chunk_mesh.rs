use crate::graphic::terrain::texture_atlas::{TextureAtlas, TextureCoordinates};
use crate::graphic::terrain::Vertex;
use crate::graphic::Context;
use math::consts::CHUNK_SIZE;
use math::positions::ChunkPos;
use wgpu::util::DeviceExt;
use world_core::block_state::AIR;
use world_core::ChunkManager;

pub struct ChunkMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl ChunkMesh {
    pub fn build_from(
        chunk_manager: &ChunkManager,
        pos: ChunkPos,
        texture_atlas: &TextureAtlas,
        context: &Context,
    ) -> Option<Self> {
        let chunk = chunk_manager.get_chunk(pos)?;
        if chunk.is_empty() {
            return None;
        }
        let top_chunk = chunk_manager.get_chunk(pos + ChunkPos::Y);
        let bottom_chunk = chunk_manager.get_chunk(pos + ChunkPos::NEG_Y);
        let west_chunk = chunk_manager.get_chunk(pos + ChunkPos::NEG_X);
        let east_chunk = chunk_manager.get_chunk(pos + ChunkPos::X);
        let north_chunk = chunk_manager.get_chunk(pos + ChunkPos::NEG_Z);
        let south_chunk = chunk_manager.get_chunk(pos + ChunkPos::Z);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        enum Face {
            Top,
            Bottom,
            West,  //x-
            East,  //X+
            North, //z-
            South, //z+
        }

        let get_block_at = |x: i32, y: i32, z: i32| {
            if x >= 0 && x < CHUNK_SIZE && y >= 0 && y < CHUNK_SIZE && z >= 0 && z < CHUNK_SIZE {
                return chunk.get_block_at(x, y, z);
            }
            if x < 0 {
                return west_chunk.map_or(AIR, |c| c.get_block_at(x + CHUNK_SIZE, y, z));
            }
            if x >= CHUNK_SIZE {
                return east_chunk.map_or(AIR, |c| c.get_block_at(x - CHUNK_SIZE, y, z));
            }
            if y < 0 {
                return bottom_chunk.map_or(AIR, |c| c.get_block_at(x, y + CHUNK_SIZE, z));
            }
            if y >= CHUNK_SIZE {
                return top_chunk.map_or(AIR, |c| c.get_block_at(x, y - CHUNK_SIZE, z));
            }
            if z < 0 {
                return north_chunk.map_or(AIR, |c| c.get_block_at(x, y, z + CHUNK_SIZE));
            }
            if z >= CHUNK_SIZE {
                return south_chunk.map_or(AIR, |c| c.get_block_at(x, y, z - CHUNK_SIZE));
            }
            AIR
        };

        //no clue why but if (0, 0, 0) is the first corner of the block in minecraft
        //then the second one is at (1, 1, -1), why the z is negative is beyond me
        let mut add_face =
            |x, y, z, face: Face, texture: TextureCoordinates, texture_index: u32| match face {
                Face::Top => {
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z - 1.0],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index,
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
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index,
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
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z - 1.0],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y, z],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index,
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
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z - 1.0],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index,
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
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z - 1.0],
                        texture_coords: [texture.x2, texture.y1],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z - 1.0],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z - 1.0],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index,
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
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y, z],
                        texture_coords: [texture.x1, texture.y1],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x + 1.0, y + 1.0, z],
                        texture_coords: [texture.x1, texture.y2],
                        texture_index,
                    });
                    vertices.push(Vertex {
                        position: [x, y + 1.0, z],
                        texture_coords: [texture.x2, texture.y2],
                        texture_index,
                    });
                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 3);
                    indices.push(vertices.len() as u32 - 2);

                    indices.push(vertices.len() as u32 - 4);
                    indices.push(vertices.len() as u32 - 2);
                    indices.push(vertices.len() as u32 - 1);
                }
            };

        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let blockstate = chunk.get_block_at(x, y, z);
                    if blockstate == AIR {
                        continue;
                    }
                    let blockstate = (blockstate - 1) as u32;

                    let texture_coordinates = texture_atlas.get_texture_coordinates();
                    let fx = x as f32;
                    let fy = y as f32;
                    let fz = z as f32;
                    if get_block_at(x, y + 1, z) == AIR {
                        add_face(fx, fy, fz, Face::Top, texture_coordinates, blockstate);
                    }
                    if get_block_at(x, y - 1, z) == AIR {
                        add_face(fx, fy, fz, Face::Bottom, texture_coordinates, blockstate);
                    }
                    if get_block_at(x - 1, y, z) == AIR {
                        add_face(fx, fy, fz, Face::West, texture_coordinates, blockstate);
                    }
                    if get_block_at(x + 1, y, z) == AIR {
                        add_face(fx, fy, fz, Face::East, texture_coordinates, blockstate);
                    }
                    if get_block_at(x, y, z - 1) == AIR {
                        add_face(fx, fy, fz, Face::North, texture_coordinates, blockstate);
                    }
                    if get_block_at(x, y, z + 1) == AIR {
                        add_face(fx, fy, fz, Face::South, texture_coordinates, blockstate);
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
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let index_count = indices.len() as u32;
        Self {
            vertex_buffer,
            index_buffer,
            index_count,
        }
    }

    pub fn draw<'pass>(&'pass self, render_pass: &mut wgpu::RenderPass<'pass>, pos_index: usize) {
        let pos_index = pos_index as u32;
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, pos_index..pos_index + 1);
    }
}
