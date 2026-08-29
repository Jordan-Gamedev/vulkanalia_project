use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PerInstanceData {
    pub model_matrix_info: u32,
    pub texture_index: u16,
    pub sampler_index: u16,
    pub mesh_metadata_index: u16,
    pub mesh_asset_id: u16,
}
