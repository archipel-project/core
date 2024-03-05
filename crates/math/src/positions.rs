use crate::consts::{CHUNK_SIZE, CHUNK_SIZE_D, CHUNK_SIZE_F};
use glam::{DVec3, IVec3, Vec3};
use std::ops::{Add, AddAssign};

/// A chunk position in the world, measured in chunks, valid from -2^27 to 2^27 - 1
pub type ChunkPos = IVec3;

/// A block position in the world, measured in blocks, valid from -2^31 to 2^31 - 1,
pub type BlockPos = IVec3;

/// A world BlockPos for Entities or other things that need to be more precise than a block, it is a combination of a chunk position and a floating point block position
/// useful for rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityPos {
    pub chunk_pos: ChunkPos,
    pub relative_pos: Vec3,
}

impl From<EntityPos> for DVec3 {
    fn from(pos: EntityPos) -> Self {
        DVec3::new(
            //I hope this is precise enough
            (pos.chunk_pos.x * CHUNK_SIZE) as f64 + pos.relative_pos.x as f64,
            (pos.chunk_pos.y * CHUNK_SIZE) as f64 + pos.relative_pos.y as f64,
            (pos.chunk_pos.z * CHUNK_SIZE) as f64 + pos.relative_pos.z as f64,
        )
    }
}

impl From<DVec3> for EntityPos {
    fn from(pos: DVec3) -> Self {
        let chunk = IVec3::new(
            (pos.x / CHUNK_SIZE_D) as i32,
            (pos.y / CHUNK_SIZE_D) as i32,
            (pos.z / CHUNK_SIZE_D) as i32,
        );
        let relative_pos = Vec3::new(
            (pos.x % CHUNK_SIZE_D) as f32,
            (pos.y % CHUNK_SIZE_D) as f32,
            (pos.z % CHUNK_SIZE_D) as f32,
        );
        Self {
            chunk_pos: chunk,
            relative_pos,
        }
    }
}

impl From<EntityPos> for BlockPos {
    fn from(pos: EntityPos) -> Self {
        BlockPos::new(
            pos.chunk_pos.x * CHUNK_SIZE + pos.relative_pos.x as i32,
            pos.chunk_pos.y * CHUNK_SIZE + pos.relative_pos.y as i32,
            pos.chunk_pos.z * CHUNK_SIZE + pos.relative_pos.z as i32,
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
            chunk_pos: chunk,
            relative_pos,
        }
    }
}

impl EntityPos {
    pub fn new(chunk: ChunkPos, relative_pos: Vec3) -> Self {
        Self {
            chunk_pos: chunk,
            relative_pos,
        }
    }

    pub fn from(x: f64, y: f64, z: f64) -> Self {
        let chunk = IVec3::new(
            (x / CHUNK_SIZE_D) as i32,
            (y / CHUNK_SIZE_D) as i32,
            (z / CHUNK_SIZE_D) as i32,
        );
        let relative_pos = Vec3::new(
            (x % CHUNK_SIZE_D) as f32,
            (y % CHUNK_SIZE_D) as f32,
            (z % CHUNK_SIZE_D) as f32,
        );
        Self {
            chunk_pos: chunk,
            relative_pos,
        }
    }

    /// reduce the relative position to the range [0, CHUNK_SIZE]
    pub fn shrink(&self) -> Self {
        let new_relative_pos = Vec3::new(
            self.relative_pos.x.rem_euclid(CHUNK_SIZE_F),
            self.relative_pos.y.rem_euclid(CHUNK_SIZE_F),
            self.relative_pos.z.rem_euclid(CHUNK_SIZE_F),
        );
        let delta_chunk = IVec3::new(
            self.relative_pos.x.div_euclid(CHUNK_SIZE_F) as i32,
            self.relative_pos.y.div_euclid(CHUNK_SIZE_F) as i32,
            self.relative_pos.z.div_euclid(CHUNK_SIZE_F) as i32,
        );
        Self {
            chunk_pos: self.chunk_pos + delta_chunk,
            relative_pos: new_relative_pos,
        }
    }

    /// try to shrink the relative position, return the last chunk pos if the chunk_position has changed, useful if entities need to be sent to another chunk
    pub fn try_shrink(&mut self) -> Option<ChunkPos> {
        let new = self.shrink();
        if new.chunk_pos != self.chunk_pos {
            *self = new;
            Some(new.chunk_pos)
        } else {
            None
        }
    }
}

impl Add<Vec3> for EntityPos {
    type Output = Self;

    /// add a vector to the relative position, doesn't shrink the relative position
    fn add(self, rhs: Vec3) -> Self::Output {
        let new_relative_pos = self.relative_pos + rhs;
        Self {
            chunk_pos: self.chunk_pos,
            relative_pos: new_relative_pos,
        }
    }
}

impl AddAssign<Vec3> for EntityPos {
    /// add a vector to the relative position, doesn't shrink the relative position
    fn add_assign(&mut self, rhs: Vec3) {
        let new = self.add(rhs);
        *self = new;
    }
}
