use glam::Vec3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct MeshMetadata {
    pub min_aabb: Vec3,
    pub max_aabb: Vec3,
    pub cull_percentage: u16,
    pub lod3_percentage: u16,
    pub lod2_percentage: u8,
    pub lod1_percentage: u8,
}

impl Eq for MeshMetadata {}

impl MeshMetadata {
    pub fn new(
        min_aabb: Vec3,
        max_aabb: Vec3,
        lod1_percentage: f32,
        lod2_percentage: f32,
        lod3_percentage: f32,
        cull_percentage: f32,
    ) -> Self {
        let cull_percentage: u16 = (cull_percentage * u16::MAX as f32) as u16;
        let lod3_percentage: u16 = (lod3_percentage * u16::MAX as f32) as u16;
        let lod2_percentage: u8 = (lod2_percentage * u8::MAX as f32) as u8;
        let lod1_percentage: u8 = (lod1_percentage * u8::MAX as f32) as u8;

        Self {
            min_aabb,
            max_aabb,
            cull_percentage,
            lod3_percentage,
            lod2_percentage,
            lod1_percentage,
        }
    }
}
