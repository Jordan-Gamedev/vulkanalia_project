use crate::engine::MeshMetadata;
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MeshBufferLayout(pub [MeshMetadata; 2048]);

impl Eq for MeshBufferLayout {}

impl Default for MeshBufferLayout {
    fn default() -> Self {
        let meshes: [MeshMetadata; 2048] = std::array::repeat(MeshMetadata {
            min_aabb: Vec3::ZERO,
            max_aabb: Vec3::ZERO,
            cull_percentage: 0,
            lod3_percentage: 0,
            lod2_percentage: 0,
            lod1_percentage: 0,
        });

        Self(meshes)
    }
}

unsafe impl Sync for MeshBufferLayout {}
unsafe impl Send for MeshBufferLayout {}
