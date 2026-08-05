#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstant {
    pub model_matrix_info: u32,
    pub texture_index: u32,
}
