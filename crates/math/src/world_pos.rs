use glam::{DVec3, IVec3, Vec3};

/// The size of a chunk in blocks, one block is 1x1x1 meters
const CHUNK_SIZE: i32 = 16;
const CHUNK_SIZE_F: f32 = CHUNK_SIZE as f32;
/// A chunk position in the world, measured in chunks, valid from -2^27 to 2^27 - 1
type ChunkPos = IVec3;

/// A block position in the world, measured in blocks, valid from -2^31 to 2^31 - 1
type BlockPos = IVec3;

/// A world BlockPos for Entities or other things that need to be more precise than a block, it is a combination of a chunk position and a floating point block position
/// useful for rendering
pub struct EntityPos {
    chunk: ChunkPos,
    relative_pos: Vec3,
}

impl From<EntityPos> for DVec3 {
    fn from(pos: EntityPos) -> Self {
        DVec3::new(
            //I hope this is precise enough
            (pos.chunk.x * CHUNK_SIZE) as f64 + pos.relative_pos.x as f64,
            (pos.chunk.y * CHUNK_SIZE) as f64 + pos.relative_pos.y as f64,
            (pos.chunk.z * CHUNK_SIZE) as f64 + pos.relative_pos.z as f64,
        )
    }
}

impl From<EntityPos> for BlockPos {
    fn from(pos: EntityPos) -> Self {
        BlockPos::new(
            pos.chunk.x * CHUNK_SIZE + pos.relative_pos.x as i32,
            pos.chunk.y * CHUNK_SIZE + pos.relative_pos.y as i32,
            pos.chunk.z * CHUNK_SIZE + pos.relative_pos.z as i32,
        )
    }
}

impl From<BlockPos> for EntityPos {
    fn from(pos: BlockPos) -> Self {
        let chunk = IVec3::new(pos.x / CHUNK_SIZE, pos.y / CHUNK_SIZE, pos.z / CHUNK_SIZE);
        let relative_pos = Vec3::new(
            pos.x as f32 % CHUNK_SIZE_F,
            pos.y as f32 % CHUNK_SIZE_F,
            pos.z as f32 % CHUNK_SIZE_F,
        );
        Self {
            chunk,
            relative_pos,
        }
    }
}

impl EntityPos {
    pub fn new(chunk: ChunkPos, relative_pos: Vec3) -> Self {
        Self {
            chunk,
            relative_pos,
        }
    }

    /// reduce the relative position to the range [0, CHUNK_SIZE]
    pub fn shrink(&self) -> Self {
        let new_relative_pos = Vec3::new(
            self.relative_pos.x % CHUNK_SIZE_F,
            self.relative_pos.y % CHUNK_SIZE_F,
            self.relative_pos.z % CHUNK_SIZE_F,
        );
        let delta_chunk = IVec3::new(
            (self.relative_pos.x / CHUNK_SIZE_F) as i32,
            (self.relative_pos.y / CHUNK_SIZE_F) as i32,
            (self.relative_pos.z / CHUNK_SIZE_F) as i32,
        );
        Self {
            chunk: self.chunk + delta_chunk,
            relative_pos: new_relative_pos,
        }
    }
}
