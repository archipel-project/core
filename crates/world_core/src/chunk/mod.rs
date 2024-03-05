mod implementation;

use crate::block_state::{BlockState, AIR};
use ctor::ctor;
use implementation::{Chunk4Bits, Chunk8Bits, ChunkNative, InMemoryChunk};
use math::positions::{BlockPos, ChunkPos};
use math::{consts::CHUNK_SIZE, IVec3};
use shared_arena::{ArenaBox, SharedArena};
use utils::memory_utils::MemorySize;

///class where all memory used by the chunk is stored, should leave longer than all the world_core loaded in memory
pub struct ChunkMemoryPool {
    chunks_native: SharedArena<ChunkNative>,
    chunks8bits: SharedArena<Chunk8Bits>,
    chunks4bits: SharedArena<Chunk4Bits>,
}

impl ChunkMemoryPool {
    pub fn new() -> Self {
        Self {
            chunks_native: SharedArena::new(),
            chunks8bits: SharedArena::new(),
            chunks4bits: SharedArena::new(),
        }
    }

    ///return the memory used and the memory pre-allocated but not used
    pub fn stats(&self) -> (MemorySize, MemorySize) {
        let (native_used, native_free) = self.chunks_native.stats();
        let (bits8_used, bits8_free) = self.chunks8bits.stats();
        let (bits4_used, bits4_free) = self.chunks4bits.stats();

        let memory_used = |native_used, bits8_used, bits4_used| {
            native_used * std::mem::size_of::<ChunkNative>()
                + bits8_used * std::mem::size_of::<Chunk8Bits>()
                + bits4_used * std::mem::size_of::<Chunk4Bits>()
        };

        let total_used = memory_used(native_used, bits8_used, bits4_used);
        let total_free = memory_used(native_free, bits8_free, bits4_free);
        (total_used.into(), total_free.into())
    }
}

enum ChunkHandle {
    ChunkEmpty,
    ChunkNative(ArenaBox<ChunkNative>),
    Chunk8bits(ArenaBox<Chunk8Bits>),
    Chunk4bits(ArenaBox<Chunk4Bits>),
}

///represent a non-empty chunk loaded in memory, this class is responsible for the memory management of the chunk as well as the chunk format
pub struct Chunk {
    position: ChunkPos,
    handle: ChunkHandle,
    //memory map and metadata can be safely added here
}

#[ctor]
pub static MEMORY_MANAGER: ChunkMemoryPool = ChunkMemoryPool::new();

impl Chunk {
    pub const SIZE: i32 = CHUNK_SIZE;

    pub fn new(position: ChunkPos) -> Self {
        Self {
            position,
            handle: ChunkHandle::ChunkEmpty,
        }
    }

    ///promote the chunk to a bigger format, if the chunk is already in the largest format, nothing happens
    ///this function take time and extend the chunk in way that make it use more memory, so it should be used carefully
    pub fn promote(&mut self) {
        match &self.handle {
            ChunkHandle::ChunkNative(_) => (),
            ChunkHandle::Chunk8bits(handle) => {
                let mut new_handle = MEMORY_MANAGER.chunks_native.alloc(ChunkNative::new());
                handle.promote_to(&mut new_handle);
                self.handle = ChunkHandle::ChunkNative(new_handle);
            }
            ChunkHandle::Chunk4bits(chunk) => {
                let mut new_handle = MEMORY_MANAGER.chunks8bits.alloc(Chunk8Bits::new());
                chunk.promote_to(&mut new_handle);
                self.handle = ChunkHandle::Chunk8bits(new_handle)
            }
            ChunkHandle::ChunkEmpty => {
                let new_handle = MEMORY_MANAGER.chunks4bits.alloc(Chunk4Bits::new()); //nothing to copy
                self.handle = ChunkHandle::Chunk4bits(new_handle)
            }
        }
    }

    ///get the blockstate at the given position
    pub fn get_block(&self, pos: BlockPos) -> BlockState {
        match self.handle {
            ChunkHandle::ChunkNative(ref chunk) => chunk.get_block(pos),
            ChunkHandle::Chunk8bits(ref chunk) => chunk.get_block(pos),
            ChunkHandle::Chunk4bits(ref chunk) => chunk.get_block(pos),
            ChunkHandle::ChunkEmpty => AIR,
        }
    }

    ///get the blockstate at the given position
    pub fn get_block_at(&self, x: i32, y: i32, z: i32) -> BlockState {
        self.get_block(BlockPos::new(x, y, z))
    }

    ///set the blockstate at the given position
    pub fn set_block(&mut self, pos: BlockPos, state: BlockState) {
        //set the blockstate at the given position can fail if the chunk is not in the right format
        while !match self.handle {
            ChunkHandle::ChunkNative(ref mut chunk) => chunk.try_set_block(pos, state),
            ChunkHandle::Chunk8bits(ref mut chunk) => chunk.try_set_block(pos, state),
            ChunkHandle::Chunk4bits(ref mut chunk) => chunk.try_set_block(pos, state),
            ChunkHandle::ChunkEmpty => false,
        } {
            self.promote();
        }
    }

    ///get the position of the chunk in the world
    pub fn position(&self) -> ChunkPos {
        self.position
    }

    ///set the blockstate at the given position, just an alias for set_block
    pub fn set_block_at(&mut self, x: i32, y: i32, z: i32, state: BlockState) {
        self.set_block(BlockPos::new(x, y, z), state);
    }

    ///return true if the chunk only contains air, it doesn't mean that the chunk with only air will always return true (because of the promotion)
    ///useful to skip operation on empty chunk
    pub fn is_empty(&self) -> bool {
        matches!(self.handle, ChunkHandle::ChunkEmpty)
    }

    ///get the AABB of the chunk in block coordinate
    pub fn get_aabb_in_block(&self) -> (IVec3, IVec3) {
        let min = self.position * CHUNK_SIZE;
        let max = min + IVec3::new(CHUNK_SIZE, CHUNK_SIZE, CHUNK_SIZE);
        (min, max)
    }
}
