use crate::engine::Buffer;
use crate::engine::PerInstanceData;
use crate::engine::SyncHandle;
use crate::engine::Visbuffer;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct CommandHandle {
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub sync_handle: SyncHandle,
    pub source_instance_buffer: Buffer<PerInstanceData>,
    pub main_camera_visbuffers: Vec<Visbuffer>,
}
