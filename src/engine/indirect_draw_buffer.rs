use crate::engine::Buffer;
use crate::engine::DeviceContext;
use crate::engine::IndirectDrawData;

use anyhow::Result;
use bytemuck::Zeroable;
use vulkanalia::prelude::v1_0::*;

const MAX_INDIRECT_DRAWS: usize = 1024;

pub struct IndirectDrawBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub mapped: *mut IndirectDrawData,
    pub capacity: usize,
}

impl IndirectDrawBuffer {
    pub fn new(device_context: &DeviceContext) -> Result<Self> {
        let size = (size_of::<IndirectDrawData>() * MAX_INDIRECT_DRAWS) as u64;
        let (buffer, memory) = Buffer::<IndirectDrawData>::create_buffer(
            device_context,
            size,
            vk::BufferUsageFlags::INDIRECT_BUFFER | vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let mapped = unsafe {
            device_context
                .device
                .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?
                .cast::<IndirectDrawData>()
        };

        unsafe {
            for i in 0..MAX_INDIRECT_DRAWS {
                *mapped.add(i) = IndirectDrawData::zeroed();
            }
        }

        Ok(Self {
            buffer,
            memory,
            mapped,
            capacity: MAX_INDIRECT_DRAWS,
        })
    }
}
