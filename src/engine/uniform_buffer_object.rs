use glam::Mat4;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct UniformBufferObject {
    pub view: Mat4,
    pub proj: Mat4,
}
