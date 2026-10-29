use glam::Affine3A;

pub struct Bone {
    pub ssbo_index: u32,
    pub parent_index: i32,
    pub local_matrix: Affine3A,
    pub inverse_bind_matrix: Affine3A,
}

impl Bone {
    fn is_root(&self) -> bool {
        self.parent_index == -1
    }

    fn get_skin_matrix(&self, world_matrix: Affine3A) -> Affine3A {
        world_matrix * self.inverse_bind_matrix
    }
}
