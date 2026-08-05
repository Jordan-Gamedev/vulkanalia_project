use crate::engine::DescriptorHandle;
use vulkanalia::prelude::v1_0::*;

pub struct RenderPipelineHandle {
    pub base_render_pass: vk::RenderPass,
    pub descriptor_handle: DescriptorHandle,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub framebuffers: Vec<vk::Framebuffer>,
}
