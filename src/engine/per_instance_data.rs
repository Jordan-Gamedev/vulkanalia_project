use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Pod, Zeroable)]
pub struct PerInstanceData {
    pub texture_index: u16,
    pub sampler_index: u16,
    pub mesh_metadata_index: u16,
    pub mesh_asset_id: u16,
}

impl Default for PerInstanceData {
    fn default() -> Self {
        Self {
            texture_index: u16::MAX,
            sampler_index: u16::MAX,
            mesh_metadata_index: u16::MAX,
            mesh_asset_id: u16::MAX,
        }
    }
}
