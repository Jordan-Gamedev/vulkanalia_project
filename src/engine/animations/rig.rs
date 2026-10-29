use crate::engine::animations::Bone;

pub struct Rig {
    pub bones: Vec<Bone>,
}

impl Rig {
    fn update_bone_matrices(&self) {}

    fn update_animation(&self) {}
}
