use crate::engine::Buffer;
use crate::engine::DeviceContext;
use crate::engine::PerInstanceData;
use anyhow::Result;
use vulkanalia::prelude::v1_0::*;

const MAX_INSTANCES: usize = 262_144;

#[derive(Clone)]
pub struct InstanceBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub mapped: *const PerInstanceData,
    pub capacity: usize,
}

unsafe impl Sync for InstanceBuffer {}
unsafe impl Send for InstanceBuffer {}

impl InstanceBuffer {
    pub fn new(device_context: &DeviceContext) -> Result<Self> {
        let size = (std::mem::size_of::<PerInstanceData>() * MAX_INSTANCES) as u64;
        let (buffer, memory) = Buffer::<PerInstanceData>::create_buffer(
            &device_context,
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let mapped = unsafe {
            device_context
                .device
                .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?
                .cast::<PerInstanceData>()
        };

        unsafe {
            for i in 0..MAX_INSTANCES {
                *mapped.add(i) = PerInstanceData {
                    model_matrix_info: u32::MAX,
                    texture_index: u32::MAX,
                    sampler_index: u32::MAX,
                    padding: u32::MAX,
                };
            }
        }

        Ok(Self {
            buffer,
            memory,
            mapped,
            capacity: MAX_INSTANCES,
        })
    }
}
