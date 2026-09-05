use crate::engine::Buffer;
use crate::engine::IndirectDrawData;
use crate::engine::PerInstanceData;
use crate::engine::SyncHandle;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct CommandHandle {
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub sync_handle: SyncHandle,
    pub indirect_draw_buffers: Vec<Buffer<IndirectDrawData>>,
    pub instance_buffers: Vec<Buffer<PerInstanceData>>,
    pub main_camera_visbuffers: Vec<Buffer<u32>>,
}
