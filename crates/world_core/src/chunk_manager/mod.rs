use std::collections::{HashMap};
use math::{AABB, I16Vec3, IVec3};
use utils::array_utils::ArrayUtils;
use crate::chunk::{Chunk, ChunkPos};

const NODE_SUBDIVISION: i32 = 8; //power of 2 are nice because they can be optimized by the compiler, this value couldn't really be changed without rewriting the tree_index_iterator function (which is a bit ugly)

///a node in the octree, it can be a leaf or a branch
trait Node {
    const LEVEL: u32; //level of the node in the octree, level 0 is the leaf
    const SIDE_CHUNK_COUNT : i32 = NODE_SUBDIVISION.pow(Self::LEVEL); //the theoretical number of chunks in a side of the node


    fn new(global_pos: IVec3) -> Self;

    ///get the AABB of the node in the world space

    fn get_aabb(&self) -> AABB;

    ///return the child at a given position, this position should be in the range [0, 8 * 2^level[
    fn get_chunk(&self, pos: IVec3) -> Option<&Chunk>;
    fn get_chunk_mut(&mut self, pos: IVec3) -> Option<&mut Chunk>;

    ///emplace a chunk at a given position, this position should be in the range [0, 8 * 2^level[
    fn emplace_chunk(&mut self, chunk : Chunk, pos : IVec3);

    ///put all loaded chunks that intersect the given AABB in the out vec
    fn get_chunk_in<'a>(&'a self, global_aabb: AABB, out: &mut Vec<&'a Chunk>);

    ///put all loaded chunks that intersect the given AABB  and that satisfy the predicate in the out vec
    fn get_chunk_with_predicate<'a>(&'a self, global_aabb: AABB, predicate: impl Fn(AABB)-> bool + Copy, out: &mut Vec<&'a Chunk>);

    ///put all loaded chunks that intersect the given AABB  and that satisfy the predicate in the out vec
    fn get_chunk_with_predicate_mut<'a>(&'a mut self, global_aabb: AABB, predicate: impl Fn(AABB)-> bool + Copy, out: &mut Vec<&'a mut Chunk>);

    ///put all loaded chunks in the node in the out vec
    fn add_all_chunks<'a>(&'a self, out: &mut Vec<&'a Chunk>);
}

///get the index of the child with local position
fn get_index_from_pos(pos : IVec3) -> usize {
    debug_assert!(pos.x < NODE_SUBDIVISION, "x to big" );
    debug_assert!(pos.y < NODE_SUBDIVISION, "y to big" );
    debug_assert!(pos.z < NODE_SUBDIVISION, "z to big" );
    debug_assert!(pos.x >= 0, "x to small" );
    debug_assert!(pos.y >= 0, "y to small" );
    debug_assert!(pos.z >= 0, "z to small" );
    (pos.x + pos.y * NODE_SUBDIVISION + pos.z * NODE_SUBDIVISION * NODE_SUBDIVISION) as usize
}

///an iterator that give the index of the children that intersect the given AABB and satisfy the given predicate
fn tree_index_iterator(global_pos: IVec3, global_aabb: AABB, child_side_chunk_count: i32, predicate : impl Fn(AABB) -> bool + Copy) -> impl Iterator<Item=usize>  {
    let get_aabb = |pos, cube_size| {
        AABB::new(pos, pos + IVec3::ONE * cube_size)
    };
    const ITER : [IVec3; 8] = [ //all the possible position of the children
        IVec3::new(0, 0, 0),
        IVec3::new(0, 0, 1),
        IVec3::new(0, 1, 0),
        IVec3::new(0, 1, 1),
        IVec3::new(1, 0, 0),
        IVec3::new(1, 0, 1),
        IVec3::new(1, 1, 0),
        IVec3::new(1, 1, 1),
    ];

    //if you got a better way to do this depending on NODE_SUBDIVISION, I'm all ears
    ITER.iter().filter_map(move |template_pos| { //first level of iteration
        let side_child_count = NODE_SUBDIVISION / 2;
        let local_pos = template_pos.clone() * side_child_count;
        let aabb = get_aabb(local_pos + global_pos, side_child_count * child_side_chunk_count);
        if !global_aabb.intersects(&aabb) || !predicate(aabb) {
            return  None;
        }


        Some(ITER.iter().filter_map(move |template_pos| { //second level of iteration
            let side_child_count = side_child_count / 2;
            let local_pos = local_pos + template_pos.clone() * side_child_count;
            let aabb = get_aabb(local_pos + global_pos, side_child_count * child_side_chunk_count);
            if !global_aabb.intersects(&aabb) || !predicate(aabb) {
                return  None;
            }


            Some(ITER.iter().filter_map(move |template_pos| { //third level of iteration
                let side_child_count = side_child_count / 2;
                assert_eq!(side_child_count, 1);
                let local_pos = local_pos + template_pos.clone() * side_child_count;
                let aabb = get_aabb(local_pos + global_pos, side_child_count * child_side_chunk_count);
                if !global_aabb.intersects(&aabb) || !predicate(aabb) {
                    return  None;
                }

                return Some(get_index_from_pos(local_pos));
            }))


        }).flatten()) //remove one level of nesting

    }).flatten() //remove one level of nesting
}

///Level 1 of the octree, can be considered as the "leaf", it contains 8^3 chunks
struct Level1 {
    global_pos: IVec3,
    children : [Option<Chunk>; NODE_SUBDIVISION.pow(3) as usize]
}

impl Level1 {
    const INIT : Option<Chunk> = None;
}

impl Node for Level1 {
    const LEVEL: u32 = 1;

    fn new(global_pos: IVec3) -> Self {
        Self {
            global_pos,
            children : [Self::INIT; NODE_SUBDIVISION.pow(3) as usize]
        }
    }

    fn get_aabb(&self) -> AABB {
        let min = self.global_pos;
        let max = min + IVec3::splat(Self::SIDE_CHUNK_COUNT);
        AABB::new(min, max)
    }

    fn get_chunk(&self, pos: IVec3) -> Option<&Chunk> {
        let index = get_index_from_pos(pos);
        self.children[index].as_ref()
    }

    fn get_chunk_mut(&mut self, pos: IVec3) -> Option<&mut Chunk> {
        let index = get_index_from_pos(pos);
        self.children[index].as_mut()
    }

    fn emplace_chunk(&mut self, chunk : Chunk, pos : IVec3) {
        let index = get_index_from_pos(pos);
        self.children[index] = Some(chunk);
    }

    fn get_chunk_in<'a>(&'a self, global_aabb: AABB, out: &mut Vec<&'a Chunk>) {
        let this_aabb = self.get_aabb();

        //fast check to see if the aabb intersect
        if !global_aabb.intersects(&this_aabb) { return; }

        if global_aabb.totally_contains(&this_aabb) {
            self.add_all_chunks(out);
            return;
        }

        //algorithm could be improved by calculating the intersection of the aabb with the node aabb, and then take the chunk that intersect this intersection
        for child in &self.children {
            if let Some(child) = child {
                let chunk_aabb = AABB::new(child.get_position(), child.get_position() + IVec3::ONE);
                if global_aabb.intersects(&chunk_aabb) {
                    out.push(child);
                }
            }
        }

    }

    fn get_chunk_with_predicate<'a>(&'a self, global_aabb: AABB, predicate: impl Fn(AABB) -> bool + Copy, out: &mut Vec<&'a Chunk>) {
        let this_aabb = self.get_aabb();

        if !global_aabb.intersects(&this_aabb) && !predicate(this_aabb) { return; }

        let iter = tree_index_iterator(self.global_pos, global_aabb, 1, predicate);
        for chunk in self.children.create_ref_iter(iter) {
            if let Some(chunk) = chunk {
                out.push(chunk);
            }
        }
    }

    fn get_chunk_with_predicate_mut<'a>(&'a mut self, global_aabb: AABB, predicate: impl Fn(AABB) -> bool + Copy, out: &mut Vec<&'a mut Chunk>) {
        let this_aabb = self.get_aabb();

        if !global_aabb.intersects(&this_aabb) && !predicate(this_aabb) { return; }

        let iter = tree_index_iterator(self.global_pos, global_aabb, 1, predicate);
        for chunk in self.children.create_mut_iter(iter) {
            if let Some(chunk) = chunk {
                out.push(chunk);
            }
        }
    }

    fn add_all_chunks<'a>(&'a self, out: &mut Vec<&'a Chunk>) {
        for child in &self.children {
            if let Some(child) = child {
                out.push(child);
            }
        }
    }
}

struct LevelN<CHILD: Node> {
    global_pos: IVec3,
    children : [Option<Box<CHILD>>; NODE_SUBDIVISION.pow(3) as usize],
}

impl<T : Node> LevelN<T> {
    const INIT : Option<Box<T>> = None;

    fn split_pos(pos : IVec3) -> (IVec3, IVec3) {
        let chunk_per_child = Self::SIDE_CHUNK_COUNT / NODE_SUBDIVISION;
        let local_pos = pos / chunk_per_child; //we shouldn't need a div_euclid here because were are working with positive numbers
        let pos_in_child = pos % chunk_per_child;
        (local_pos, pos_in_child)
    }
}

impl<T : Node> Node for LevelN<T> {
    const LEVEL: u32 = T::LEVEL + 1;

    fn new(global_pos: IVec3) -> Self {
        Self {
            global_pos,
            children : [Self::INIT; NODE_SUBDIVISION.pow(3) as usize]
        }
    }

    fn get_aabb(&self) -> AABB {
        let min = self.global_pos;
        let max = min + IVec3::splat(Self::SIDE_CHUNK_COUNT);
        AABB::new(min, max)
    }

    fn get_chunk(&self, pos: IVec3) -> Option<&Chunk> {
        let (local_pos, pos_in_child) = Self::split_pos(pos);
        let index = get_index_from_pos(local_pos);
        self.children[index].as_ref().and_then(|child| child.get_chunk(pos_in_child))
    }

    fn get_chunk_mut(&mut self, pos: IVec3) -> Option<&mut Chunk> {
        let (local_pos, pos_in_child) = Self::split_pos(pos);
        let index = get_index_from_pos(local_pos);
        self.children[index].as_mut().and_then(|child| child.get_chunk_mut(pos_in_child))
    }

    fn emplace_chunk(&mut self, chunk: Chunk, pos: IVec3) {
        let (local_pos, pos_in_child) = Self::split_pos(pos);
        let index = get_index_from_pos(local_pos);

        if let Some(child) = &mut self.children[index] {
            child.emplace_chunk(chunk, pos_in_child);
        } else {
            let global_pos = self.global_pos + local_pos * T::SIDE_CHUNK_COUNT;
            let mut child = Box::new(T::new(global_pos));
            child.emplace_chunk(chunk, pos_in_child);
            self.children[index] = Some(child);
        }
    }

    fn get_chunk_in<'a>(&'a self, global_aabb: AABB, out: &mut Vec<&'a Chunk>) {
        //if the local_aabb totally contains the node, we can put all the chunks in the out vec
        let this_aabb = self.get_aabb();

        if !global_aabb.intersects(&this_aabb) { return; }

        if global_aabb.totally_contains(&this_aabb) {
            self.add_all_chunks(out);
            return;
        }

        for child in &self.children {
            if let Some(child) = child {
                child.get_chunk_in(global_aabb, out);
            }
        }
    }

    fn get_chunk_with_predicate<'a>(&'a self, global_aabb: AABB, predicate: impl Fn(AABB) -> bool + Copy, out: &mut Vec<&'a Chunk>) {
        let this_aabb = self.get_aabb();

        if !global_aabb.intersects(&this_aabb) && !predicate(this_aabb) { return; }

        let iter = tree_index_iterator(self.global_pos, global_aabb, Self::SIDE_CHUNK_COUNT, predicate);
        for child in self.children.create_ref_iter(iter) {
            if let Some(child) = child {
                child.get_chunk_with_predicate(global_aabb, predicate, out);
            }
        }
    }

    fn get_chunk_with_predicate_mut<'a>(&'a mut self, global_aabb: AABB, predicate: impl Fn(AABB) -> bool + Copy, out: &mut Vec<&'a mut Chunk>) {
        let this_aabb = self.get_aabb();

        if !global_aabb.intersects(&this_aabb) && !predicate(this_aabb) { return; }

        let iter = tree_index_iterator(self.global_pos, global_aabb, T::SIDE_CHUNK_COUNT, predicate);
        for child in self.children.create_mut_iter(iter) {
            if let Some(child) = child {
                child.get_chunk_with_predicate_mut(global_aabb, predicate, out);
            }
        }
    }

    fn add_all_chunks<'a>(&'a self, out: &mut Vec<&'a  Chunk>) {
        for child in &self.children {
            if let Some(child) = child {
                child.add_all_chunks(out);
            }
        }
    }
}

type Level2 = LevelN<Level1>;
type Level3 = LevelN<Level2>;
type Level4 = LevelN<Level3>;

///a section is a 4096 chunks wide cube
type Section = Level4;

///this chunks manager cut the world in section of 4096 chunks, it has some cool properties:
///for all 32bits blockState position, there is a unique 16 bits region position, because :
/// WorldSize / (ChunkSize * RegionSize) = 2^32 / (2^4 * 2^16) = 2^16
/// So the coordinates of the section can be stored in the hash map is the 16 most significant bits of the 32bits coordinates.
/// The 16 least significant bits of the 32bits coordinates are the coordinates of the chunk in the region.
///
/// An HashMap isn't perfect when it comes to dense spatial data, but a QuadTree is.
/// However, a QuadTree is very bad when data is sparse, and it's the case here in some scale.
///
/// Using both an Octree and an HashMap seems to be the best solution. The HashMap will be used to store the regions and deal with the huge size of the world, and the QuadTree will be used to store the chunks with a good access locality.
///
///the Octree have to store 4096 chunks^3, So I chose to split each node in 512 children (8^3), which gives us a depth of 3.
///the Octree also make chunk insertion and deletion pretty fast, at least faster than in a big HashMap.
pub struct ChunkManager {
    section_map: HashMap<I16Vec3, Section> //using an octree to store the entire world would require 11 level of depth, which is a lot, the hashmap skip 6 level of depth, where the nodes are sparse and the hashmap is more efficient
}

impl ChunkManager {
    pub fn new() -> Self {
        Self {
            section_map: HashMap::new()
        }
    }

    pub fn insert_chunk(&mut self, chunk : Chunk) {
        let pos = chunk.get_position();
        let region_pos = pos.div_euclid(IVec3::splat(Section::SIDE_CHUNK_COUNT)).as_i16vec3(); //euclid division is important here, else the sign of the number will be wrong
        let local_pos = pos.rem_euclid(IVec3::splat(Section::SIDE_CHUNK_COUNT));

        if let Some(section) = self.section_map.get_mut(&region_pos) {
            section.emplace_chunk(chunk, local_pos);
        } else {
            let global_pos = region_pos.as_ivec3() * Section::SIDE_CHUNK_COUNT;
            let mut section = Section::new(global_pos);
            section.emplace_chunk(chunk, local_pos);
            self.section_map.insert(region_pos, section);
        }
    }

    pub fn get_chunk(&self, pos : ChunkPos) -> Option<&Chunk> {
        let region_pos = pos.div_euclid(IVec3::splat(Section::SIDE_CHUNK_COUNT)).as_i16vec3();
        let local_pos = pos.rem_euclid(IVec3::splat(Section::SIDE_CHUNK_COUNT));
        if let Some(section) = self.section_map.get(&region_pos) {
            section.get_chunk(local_pos)
        } else {
            None
        }
    }

    pub fn get_chunk_mut(&mut self, pos : ChunkPos) -> Option<&mut Chunk> {
        let region_pos = pos.div_euclid(IVec3::splat(Section::SIDE_CHUNK_COUNT)).as_i16vec3();
        let local_pos = pos.rem_euclid(IVec3::splat(Section::SIDE_CHUNK_COUNT));
        if let Some(section) = self.section_map.get_mut(&region_pos) {
            section.get_chunk_mut(local_pos)
        } else {
            None
        }
    }

    ///get all loaded chunks in the given AABB
    pub fn get_chunks_in(&self, chunk_aabb : AABB) -> Vec<&Chunk> {
        let mut chunks = Vec::with_capacity(chunk_aabb.get_volume() as usize);

        self.section_map.iter().for_each(|(pos, section)| {
            let section_aabb = AABB::new(pos.as_ivec3() * Section::SIDE_CHUNK_COUNT, (pos.as_ivec3() + IVec3::ONE) * Section::SIDE_CHUNK_COUNT);
            if let Some(intersection) = chunk_aabb.get_intersection(&section_aabb) {
                section.get_chunk_in(intersection, &mut chunks);
            }
        });
        chunks
    }

    ///return all loaded chunks that intersect the given AABB  and that satisfy the predicate
    pub fn get_chunk_with_predicate(&self, chunk_aabb: AABB, predicate: impl Fn(AABB) -> bool + Copy) -> Vec<&Chunk> {
        let mut chunks = Vec::with_capacity(chunk_aabb.get_volume() as usize);

        self.section_map.iter().for_each(|(pos, section)| {
            let section_aabb = AABB::new(pos.as_ivec3() * Section::SIDE_CHUNK_COUNT, (pos.as_ivec3() + IVec3::ONE) * Section::SIDE_CHUNK_COUNT);
            if let Some(intersection) = chunk_aabb.get_intersection(&section_aabb) {
                section.get_chunk_with_predicate(intersection, predicate, &mut chunks);
            }
        });
        chunks
    }

    ///return all loaded chunks that intersect the given AABB  and that satisfy the predicate
    pub fn get_chunk_with_predicate_mut(&mut self, chunk_aabb: AABB, predicate: impl Fn(AABB) -> bool + Copy) -> Vec<&mut Chunk> {
        let mut chunks = Vec::with_capacity(chunk_aabb.get_volume() as usize);

        self.section_map.iter_mut().for_each(|(pos, section)| {
            let section_aabb = AABB::new(pos.as_ivec3() * Section::SIDE_CHUNK_COUNT, (pos.as_ivec3() + IVec3::ONE) * Section::SIDE_CHUNK_COUNT);
            if let Some(intersection) = chunk_aabb.get_intersection(&section_aabb) {
                section.get_chunk_with_predicate_mut(intersection, predicate, &mut chunks);
            }
        });
        chunks
    }
}