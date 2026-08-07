use crate::engine::IndirectDrawBuffer;
use crate::engine::InstanceBuffer;
use crate::engine::SyncHandle;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct CommandHandle {
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub sync_handle: SyncHandle,
    pub indirect_draw_buffer: IndirectDrawBuffer,
    pub instance_buffer: InstanceBuffer,
}
