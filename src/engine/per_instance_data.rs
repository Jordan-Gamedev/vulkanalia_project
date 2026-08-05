use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct PerInstanceData {
    pub model_matrix_info: u32,
    pub texture_index: u32,
    pub sampler_index: u32,
    pub padding: u32,
}
