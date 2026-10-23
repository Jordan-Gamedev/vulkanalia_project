use crate::engine::Buffer;
use vulkanalia::prelude::v1_0::*;

use super::device_context::DeviceContext;

#[derive(Default)]
pub struct InFlightBuffers<T: Clone + std::fmt::Debug + Default> {
    pub buffers: Vec<Buffer<T>>,
    updated_buffer_frame_index: usize,
    num_frames_needing_update: usize,
    max_frames_in_flight: usize,
    current_frame_index: usize,
}

impl<T: Clone + std::fmt::Debug + Default> InFlightBuffers<T> {
    pub fn new(
        max_frames_in_flight: usize,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        initial_capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        alloc_dealloc_threshold: u32,
        initial_contents: Vec<T>,
        should_preserve_indices: bool,
    ) -> Self {
        let mut buffers: Vec<Buffer<T>> = Vec::new();
        for _ in 0..max_frames_in_flight {
            buffers.push(Buffer::new(
                device_context,
                command_pool,
                initial_capacity,
                usage | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
                properties,
                alloc_dealloc_threshold,
                initial_contents.clone(),
                should_preserve_indices,
            ));
        }

        Self {
            buffers,
            updated_buffer_frame_index: 0,
            num_frames_needing_update: 0,
            max_frames_in_flight,
            current_frame_index: 0,
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        self.buffers.iter_mut().for_each(|b| b.destroy(device));
    }

    pub fn get_current(&self) -> &Buffer<T> {
        &self.buffers[self.current_frame_index]
    }

    pub fn get_current_mut(&mut self) -> &mut Buffer<T> {
        &mut self.buffers[self.current_frame_index]
    }

    pub fn temp_get_current_frame(&self) -> usize {
        self.current_frame_index
    }

    // Call if you want the other frame's versions of the buffer to be overwritten by data from this buffer
    pub fn signal_buffer_change_propagation(&mut self) {
        self.num_frames_needing_update = self.buffers.len() - 1;
    }

    // If changed, at the beginning of the frame replace this buffer with the previous frame's buffer, which is the latest version of the buffer
    pub fn update(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        descriptor_sets: &Vec<vk::DescriptorSet>,
        descriptor_binding: u32,
    ) {
        self.current_frame_index = (self.current_frame_index + 1) % self.max_frames_in_flight;
        if self.num_frames_needing_update > 0 {
            let (
                updated_buffer,
                updated_element_count,
                updated_element_capacity,
                available_indices,
                contents,
            ) = {
                let updated_buffer = self.buffers.get(self.updated_buffer_frame_index).unwrap();
                (
                    updated_buffer.buffer,
                    updated_buffer.element_count,
                    updated_buffer.element_capacity,
                    updated_buffer.available_indices.clone(),
                    updated_buffer
                        .get_buffer_items(device_context, command_pool, true)
                        .unwrap(),
                )
            };
            let current_buffer = self.buffers.get_mut(self.current_frame_index).unwrap();

            if updated_element_capacity == current_buffer.element_capacity {
                Buffer::<T>::copy_buffer(
                    device_context,
                    command_pool,
                    updated_buffer,
                    current_buffer.buffer,
                    current_buffer.element_capacity as u64,
                )
                .unwrap();

                current_buffer.element_count = updated_element_count;
                current_buffer.available_indices = available_indices;
            } else {
                current_buffer.recreate(
                    device_context,
                    command_pool,
                    contents,
                    descriptor_sets[self.current_frame_index],
                    descriptor_binding,
                );

                current_buffer.element_count = updated_element_count;
                current_buffer.available_indices = available_indices;
            }

            self.num_frames_needing_update -= 1;
        }

        self.updated_buffer_frame_index = self.current_frame_index;
    }
}
