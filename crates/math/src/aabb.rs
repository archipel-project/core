use glam::IVec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AABB {
    pub(crate) min: IVec3,
    pub(crate) max: IVec3,
}

impl AABB {
    pub fn new(min: IVec3, max: IVec3) -> Self {
        debug_assert!(min.x < max.x);
        debug_assert!(min.y < max.y);
        debug_assert!(min.z < max.z);
        Self { min, max }
    }

    pub fn safe_new(min: IVec3, max: IVec3) -> Self {
        let min = min.min(max);
        let max = min.max(max);
        Self { min, max }
    }

    pub fn contains(&self, pos: IVec3) -> bool {
        pos.x >= self.min.x
            && pos.x <= self.max.x
            && pos.y >= self.min.y
            && pos.y <= self.max.y
            && pos.z >= self.min.z
            && pos.z <= self.max.z
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }

    pub fn get_intersection(&self, other: &AABB) -> Option<AABB> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min.x < max.x && min.y < max.y && min.z < max.z {
            Some(AABB::new(min, max))
        } else {
            None
        }
    }

    pub fn totally_contains(&self, other: &AABB) -> bool {
        self.min.x <= other.min.x
            && self.max.x >= other.max.x
            && self.min.y <= other.min.y
            && self.max.y >= other.max.y
            && self.min.z <= other.min.z
            && self.max.z >= other.max.z
    }

    pub fn corners(&self) -> [IVec3; 8] {
        [
            IVec3::new(self.min.x, self.min.y, self.min.z),
            IVec3::new(self.min.x, self.min.y, self.max.z),
            IVec3::new(self.min.x, self.max.y, self.min.z),
            IVec3::new(self.min.x, self.max.y, self.max.z),
            IVec3::new(self.max.x, self.min.y, self.min.z),
            IVec3::new(self.max.x, self.min.y, self.max.z),
            IVec3::new(self.max.x, self.max.y, self.min.z),
            IVec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    pub fn get_volume(&self) -> i32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    pub fn size(&self) -> IVec3 {
        self.max - self.min
    }

    pub fn is_unit(&self) -> bool {
        let size = self.size();
        size.x == 1 && size.y == 1 && size.z == 1
    }

    pub fn clamp(&self, pos: IVec3) -> IVec3 {
        pos.clamp(self.min, self.max)
    }
}
