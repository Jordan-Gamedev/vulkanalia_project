use crate::engine::InFlightBuffers;
use crate::engine::IndirectDrawData;
use crate::engine::PerInstanceData;
use crate::engine::SyncHandle;
use vulkanalia::prelude::v1_0::*;

pub struct CommandHandle {
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub sync_handle: SyncHandle,
    pub indirect_draw_buffers: InFlightBuffers<IndirectDrawData>,
    pub instance_buffers: InFlightBuffers<PerInstanceData>,
    pub main_camera_visbuffers: InFlightBuffers<u32>,
}
