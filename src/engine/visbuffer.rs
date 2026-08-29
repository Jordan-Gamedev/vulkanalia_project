use vulkanalia::prelude::v1_0::*;

use crate::engine::Buffer;
use crate::engine::DeviceContext;
use crate::engine::IndirectDrawData;
use crate::engine::PerInstanceData;

#[derive(Clone)]
pub struct Visbuffer {
    pub indirect_draw_buffer: Buffer<IndirectDrawData>,
    pub instance_buffer: Buffer<PerInstanceData>,
}

impl Visbuffer {
    pub fn new(
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        instance_capacity: usize,
    ) -> Self {
        let indirect_draw_buffer = Buffer::<IndirectDrawData>::new(
            device_context,
            command_pool,
            8192,
            vk::BufferUsageFlags::INDIRECT_BUFFER | vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            0,
            Vec::new(),
            false,
        );

        let instance_buffer = Buffer::<PerInstanceData>::new(
            device_context,
            command_pool,
            instance_capacity as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            0,
            Vec::new(),
            false,
        );

        Self {
            indirect_draw_buffer: indirect_draw_buffer,
            instance_buffer: instance_buffer,
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        self.indirect_draw_buffer.destroy(device);
        self.instance_buffer.destroy(device);
    }
}
