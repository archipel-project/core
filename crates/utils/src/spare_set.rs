type Uint = u32;

/// A unique identifier for a resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(Uint);

impl Id {
    pub fn raw(&self) -> Uint {
        self.0
    }

    fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

pub struct IdTracker {
    free: Vec<Uint>,
    next: Uint,
}

/// Allocate the smallest possible ID at each request. To make SparseSet more efficient
impl IdTracker {
    pub fn new() -> Self {
        Self {
            free: Vec::new(),
            next: 0,
        }
    }

    pub fn alloc(&mut self) -> Id {
        if let Some(id) = self.free.pop() {
            Id(id)
        } else {
            let id = self.next;
            self.next += 1;
            Id(id)
        }
    }

    pub fn free(&mut self, id: Id) {
        self.free.push(id.raw());
    }
}

struct DenseNode<T> {
    sparse_pos: Id,
    value: T,
}

///T is the type of the elements, U is the type of the ID
pub struct SparseSet<T> {
    dense: Vec<DenseNode<T>>,
    sparse: Vec<Uint>, // the index of the dense array is the ID
}

///this SparseSet use Max as the Empty value
impl<T> SparseSet<T> {
    const EMPTY: Uint = Uint::MAX;

    ///create a new SparseSet
    pub fn new() -> Self {
        Self {
            dense: Vec::new(),
            sparse: Vec::new(),
        }
    }

    ///create a new SparseSet that can hold at least the given number of elements without reallocating the dense array
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            dense: Vec::with_capacity(capacity),
            sparse: Vec::new(),
        }
    }

    ///only retain the elements that satisfy the given predicate, in other words, remove all the elements that do not satisfy the given predicate
    /*pub fn retain(&mut self, mut f: impl FnMut(Id, &mut T) -> bool) {
        let mut i = 0;
        while i < self.dense.len() {
            let id = Id(self.dense[i].sparse_pos);
            if !f(id, &mut self.dense[i].value) {
                self.remove(id);
            } else {
                i += 1;
            }
        }
    }*/

    //set the sparse array at the given ID to the given dense position
    fn set_sparse_id(&mut self, id: Id, dense_pos: Uint) {
        assert!(
            dense_pos < Self::EMPTY,
            "too many elements in the SparseSet, the maximum number of elements is {}",
            Self::EMPTY - 1
        );
        if id.as_usize() >= self.sparse.len() {
            self.sparse.resize(id.as_usize() + 1, Self::EMPTY);
        }
        self.sparse[id.as_usize()] = dense_pos;
    }

    fn sparse_get_dense_pos(&self, id: Id) -> Uint {
        self.sparse
            .get(id.as_usize())
            .unwrap_or(&Self::EMPTY)
            .clone()
    }

    ///insert the value at the given ID, if the map did have this key present, the value is updated, and the old value is returned
    pub fn insert(&mut self, id: Id, value: T) -> Option<T> {
        let dense_pos = self.sparse_get_dense_pos(id);

        let new_node = DenseNode {
            sparse_pos: id,
            value,
        };

        if dense_pos == Self::EMPTY {
            let dense_pos = self.dense.len() as Uint;
            self.dense.push(new_node);
            self.set_sparse_id(id, dense_pos);
            assert!(id.raw() < self.sparse.len() as Uint);
            None
        } else {
            let old_node = &mut self.dense[dense_pos as usize];
            let old_node = std::mem::replace(old_node, new_node);
            assert!(id.raw() < self.sparse.len() as Uint);
            Some(old_node.value)
        }
    }

    ///get the element at the given ID if it exists
    pub fn get(&self, id: Id) -> Option<&T> {
        let dense_pos = self.sparse_get_dense_pos(id);
        if dense_pos == Self::EMPTY {
            None
        } else {
            Some(&self.dense[dense_pos as usize].value)
        }
    }

    ///get the element at the given ID if it exists
    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        let dense_pos = self.sparse_get_dense_pos(id);
        if dense_pos == Self::EMPTY {
            return None;
        } else {
            Some(&mut self.dense[dense_pos as usize].value)
        }
    }

    ///Remove the element at the given ID if it exists, return it
    pub fn remove(&mut self, id: Id) -> Option<T> {
        if self.dense.is_empty() {
            //if there is nothing to remove...
            return None;
        }

        let dense_pos = self.sparse_get_dense_pos(id);
        if dense_pos == Self::EMPTY {
            //if not stored by the SparseSet
            return None;
        }

        if dense_pos == self.dense.len() as Uint - 1 {
            //if the element is the last one
            let dense_node = self.dense.pop().unwrap();
            self.sparse[id.as_usize()] = Self::EMPTY;
            Some(dense_node.value)
        } else {
            let last_element_sparse_pos = self.dense.last().unwrap().sparse_pos; //this is the position of the last element in the sparse array
            let dense_node = self.dense.swap_remove(dense_pos as usize);

            //that mean the last element is now at dense_pos, so we need to update its position in the sparse array
            self.sparse[id.as_usize()] = Self::EMPTY;
            self.sparse[last_element_sparse_pos.as_usize()] = dense_pos; //don't know funking why but last_element_sparse_pos is somehow sometimes out of bounds
            Some(dense_node.value)
        }
    }

    ///get the number of elements in the SparseSet
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    ///get the capacity of the SparseSet
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }

    ///iterate over the elements of the SparseSet, the order is not specified, but it is guaranteed that all the elements will be visited once
    ///iterating over the SparseSet take O(len) time
    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.dense.iter().map(|node| {
            let id = node.sparse_pos;
            (id, &node.value)
        })
    }

    #[cfg(test)]
    pub fn assert_sparse_valid(&self) {
        for (dense_pos, dense_node) in self.dense.iter().enumerate() {
            let dense_pos_from_sparse = self.sparse_get_dense_pos(dense_node.sparse_pos);
            assert_eq!(dense_pos, dense_pos_from_sparse as usize);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::spare_set::Id;
    use crate::spare_set::SparseSet;
    #[test]
    pub fn main() {
        let mut sparse_set = SparseSet::new();

        for i in (0..100).rev() {
            let id = Id(i);
            assert!(sparse_set.insert(id, i).is_none());
        }

        sparse_set.assert_sparse_valid();

        for i in (0..100).rev() {
            let id = Id(i);
            assert_eq!(sparse_set.get(id), Some(&i));
        }

        sparse_set.assert_sparse_valid();
        sparse_set.remove(Id(0));
        sparse_set.assert_sparse_valid();

        for i in 0..200 {
            let id = Id(i);
            sparse_set.remove(id);
        }

        sparse_set.assert_sparse_valid();
    }
}
