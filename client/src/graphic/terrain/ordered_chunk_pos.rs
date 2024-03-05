use math::positions::ChunkPos;
use std::cmp::Ordering;

pub struct OrderedChunkPos(pub ChunkPos);

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
