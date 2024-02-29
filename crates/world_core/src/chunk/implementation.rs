use crate::block_state::{BlockState, AIR};
use crate::chunk::BlockPos;
use math::consts::CHUNK_SIZE;

///a common interface for all types of world_core in memory
pub trait InMemoryChunk {
    fn get_block(&self, pos: BlockPos) -> BlockState;
    ///return false if the set failed, in this case, the chunk should be promoted and the function should be called again
    fn try_set_block(&mut self, pos: BlockPos, state: BlockState) -> bool;
}

///the air index is used as a magical value to indicate that the palette entry is not used
const AVAILABLE_PALETTE_ENTRY: BlockState = AIR;

///stores blockStates without any compression. There is no limit of blockState Variants.
///use 8192 bytes of memory
pub struct ChunkNative {
    blocks: [BlockState; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
}

impl ChunkNative {
    pub fn new() -> ChunkNative {
        ChunkNative {
            blocks: [AVAILABLE_PALETTE_ENTRY; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }
}

impl InMemoryChunk for ChunkNative {
    fn get_block(&self, pos: BlockPos) -> BlockState {
        assert!(pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE && pos.z < CHUNK_SIZE);
        self.blocks[(pos.x + pos.y * CHUNK_SIZE + pos.z * CHUNK_SIZE * CHUNK_SIZE) as usize]
    }

    fn try_set_block(&mut self, pos: BlockPos, state: BlockState) -> bool {
        assert!(pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE && pos.z < CHUNK_SIZE);
        self.blocks[(pos.x + pos.y * CHUNK_SIZE + pos.z * CHUNK_SIZE * CHUNK_SIZE) as usize] =
            state;
        true
    }
}

///a common interface for all types of world_core using palette compression
pub trait PaletteChunk {
    fn corresponding_palette_index(&self, state: BlockState) -> Option<u8>;
    fn get_or_create_palette_index(&mut self, state: BlockState) -> Option<u8>;
    fn get_block_state_from_index(&self, palette_index: u8) -> BlockState;
}

///stores blockStates on 8bits. There is a limit of 256 blockState Variants.
///use 47% less memory than NativeChunk (4352 bytes vs 8192 bytes)
pub struct Chunk8Bits {
    palette: [BlockState; 255], //256 is the size of an u8 - 1 for the air, we could use a Vec<BlockState> but it might be less efficient since it would be allocated on the heap
    blocks: [u8; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
}

impl Chunk8Bits {
    pub fn new() -> Chunk8Bits {
        Chunk8Bits {
            palette: [AVAILABLE_PALETTE_ENTRY; 255],
            blocks: [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    pub fn promote_to(&self, native_chunk: &mut ChunkNative) {
        for (i, palette_index) in self.blocks.iter().enumerate() {
            native_chunk.blocks[i] = self.get_block_state_from_index(*palette_index);
        }
    }
}

impl PaletteChunk for Chunk8Bits {
    fn corresponding_palette_index(&self, state: BlockState) -> Option<u8> {
        if state == AIR {
            return Some(0); //0 is the static palette_index of air
        }
        for i in 0..self.palette.len() {
            if self.palette[i] == state {
                return Some(i as u8 + 1); //+1 because 0 is air
            }
        }
        None
    }

    fn get_or_create_palette_index(&mut self, state: BlockState) -> Option<u8> {
        if let Some(palette_index) = self.corresponding_palette_index(state) {
            return Some(palette_index);
        }

        for i in 0..self.palette.len() {
            if self.palette[i] == 0 {
                //0 means empty and can be used
                self.palette[i] = state;
                return Some(i as u8 + 1); //+1 because 0 is air
            }
        }

        //we should try to add a mechanism to free palette_index when the block is removed !

        None
    }

    fn get_block_state_from_index(&self, palette_index: u8) -> BlockState {
        if palette_index == 0 {
            return AIR;
        }
        self.palette[palette_index as usize - 1] // -1 because 0 is air
    }
}

impl InMemoryChunk for Chunk8Bits {
    fn get_block(&self, pos: BlockPos) -> BlockState {
        assert!(pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE && pos.z < CHUNK_SIZE);
        let palette_index =
            self.blocks[(pos.x + pos.y * CHUNK_SIZE + pos.z * CHUNK_SIZE * CHUNK_SIZE) as usize];
        self.get_block_state_from_index(palette_index)
    }

    fn try_set_block(&mut self, pos: BlockPos, state: BlockState) -> bool {
        assert!(pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE && pos.z < CHUNK_SIZE);
        let get_or_create_palette_index = self.get_or_create_palette_index(state);
        if let Some(palette_index) = get_or_create_palette_index {
            self.blocks[(pos.x + pos.y * CHUNK_SIZE + pos.z * CHUNK_SIZE * CHUNK_SIZE) as usize] =
                palette_index;
            return true;
        }
        false
    }
}

/// stores blockStates on 4bits. There is a limit of 15 blockState Variants.
/// use 74% less memory than NativeChunk (2063 bytes vs 8192 bytes)
pub struct Chunk4Bits {
    palette: [BlockState; 15], //16 is the size of an u8 - 1 for the air, we could use a Vec<BlockState> but it might be less efficient since it would be allocated on the heap
    blocks: [u8; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE / 2) as usize], //4 bits per block u4 doesn't exist in rust so we use u8...
}

impl Chunk4Bits {
    pub fn new() -> Self {
        Self {
            palette: [AVAILABLE_PALETTE_ENTRY; 15], // a bit tricky, we use the fact that air is always 0, but in fact, we set two values at a time
            blocks: [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE / 2) as usize],
        }
    }

    pub fn promote_to(&self, chunk8bits: &mut Chunk8Bits) {
        //copy the palette
        for (i, blockstate) in self.palette.iter().enumerate() {
            chunk8bits.palette[i] = *blockstate;
        }
        //copy the blocks
        for (i, block) in self.blocks.iter().enumerate() {
            let first_half = block & 0b1111;
            let second_half = block >> 4;
            chunk8bits.blocks[i * 2] = first_half;
            chunk8bits.blocks[i * 2 + 1] = second_half;
        }
    }
}

impl PaletteChunk for Chunk4Bits {
    fn corresponding_palette_index(&self, state: BlockState) -> Option<u8> {
        if state == AIR {
            return Some(0); //0 is the static palette_index of air
        }
        for i in 0..self.palette.len() {
            if self.palette[i] == state {
                return Some(i as u8 + 1); //+1 because 0 is air
            }
        }
        None
    }

    fn get_or_create_palette_index(&mut self, state: BlockState) -> Option<u8> {
        if let Some(palette_index) = self.corresponding_palette_index(state) {
            return Some(palette_index);
        }

        for i in 0..self.palette.len() {
            if self.palette[i] == 0 {
                //0 means empty and can be used
                self.palette[i] = state;
                return Some(i as u8 + 1); //+1 because 0 is air
            }
        }

        //we should try to add a mechanism to free palette_index when the block is removed !
        None
    }

    fn get_block_state_from_index(&self, palette_index: u8) -> BlockState {
        if palette_index == 0 {
            return AIR;
        }
        self.palette[palette_index as usize - 1] // -1 because 0 is air
    }
}

impl InMemoryChunk for Chunk4Bits {
    fn get_block(&self, pos: BlockPos) -> BlockState {
        assert!(pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE && pos.z < CHUNK_SIZE);

        let linear_coord = pos.x + pos.y * CHUNK_SIZE + pos.z * CHUNK_SIZE * CHUNK_SIZE;
        let array_index = linear_coord >> 1; //divide by 2
        let is_first_half = linear_coord & 1 == 0; //modulo 2

        //read the good half of the byte
        let palette_index = if is_first_half {
            self.blocks[array_index as usize] & 0b1111
        } else {
            self.blocks[array_index as usize] >> 4
        };

        self.get_block_state_from_index(palette_index)
    }

    fn try_set_block(&mut self, pos: BlockPos, state: BlockState) -> bool {
        assert!(pos.x < CHUNK_SIZE && pos.y < CHUNK_SIZE && pos.z < CHUNK_SIZE);
        let get_or_create_palette_index = self.get_or_create_palette_index(state);
        if let Some(palette_index) = get_or_create_palette_index {
            let linear_coord = pos.x + pos.y * CHUNK_SIZE + pos.z * CHUNK_SIZE * CHUNK_SIZE;
            let array_index = linear_coord >> 1; // divide by 2
            let is_first_half = linear_coord & 1 == 0; // modulo 2

            //set the good half of the byte
            if is_first_half {
                self.blocks[array_index as usize] =
                    (self.blocks[array_index as usize] & 0b11110000) | palette_index;
            } else {
                self.blocks[array_index as usize] =
                    (self.blocks[array_index as usize] & 0b00001111) | (palette_index << 4);
            }
            return true;
        }
        false
    }
}
