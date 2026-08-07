#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::Result;
use core::slice;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

use crate::engine::vulkan_renderer::{
    begin_single_time_commands, end_single_time_commands, get_memory_type_index,
};

use super::device_context::DeviceContext;

#[derive(Clone, Default)]
pub struct Buffer<T: Clone + std::fmt::Debug + Default> {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub mapped: *const T,
    pub available_indices: std::collections::BTreeSet<u32>,
    pub element_count: u32,
    pub element_capacity: u32,
    pub alloc_dealloc_threshold: u32,
    pub usage: vk::BufferUsageFlags,
    pub properties: vk::MemoryPropertyFlags,
    pub is_host_visible: bool,
}

unsafe impl<T: Clone + std::fmt::Debug + Default> Sync for Buffer<T> {}
unsafe impl<T: Clone + std::fmt::Debug + Default> Send for Buffer<T> {}

impl<T: Clone + std::fmt::Debug + Default> Buffer<T> {
    pub fn new(
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        initial_capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        alloc_dealloc_threshold: u32,
        initial_contents: Vec<T>,
    ) -> Self {
        unsafe {
            let device = device_context.device.clone();

            // If the buffer needs to be initialized with values and it isn't cpu accessible, use staging buffer
            if properties.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
                // Create (staging)

                let (staging_buffer, staging_buffer_memory) = Buffer::<T>::create_buffer(
                    &device_context,
                    initial_capacity as u64,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                )
                .unwrap();

                // Copy (staging)

                let mapped = device
                    .map_memory(
                        staging_buffer_memory,
                        0,
                        initial_capacity,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap()
                    .cast();
                memcpy(initial_contents.as_ptr(), mapped, initial_contents.len());
                device.unmap_memory(staging_buffer_memory);

                // Create (device local)

                let usage =
                    vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST | usage;
                let (device_buffer, device_buffer_memory) = Buffer::<T>::create_buffer(
                    &device_context,
                    initial_capacity,
                    usage,
                    properties,
                )
                .unwrap();

                // Copy (device local)

                Buffer::<T>::copy_buffer(
                    &device_context,
                    command_pool,
                    staging_buffer,
                    device_buffer,
                    initial_capacity,
                )
                .unwrap();

                device.destroy_buffer(staging_buffer, None);
                device.free_memory(staging_buffer_memory, None);

                Self {
                    buffer: device_buffer,
                    memory: device_buffer_memory,
                    mapped: std::ptr::null_mut(),
                    available_indices: std::collections::BTreeSet::new(),
                    element_count: initial_contents.len() as u32,
                    element_capacity: initial_capacity as u32,
                    alloc_dealloc_threshold: alloc_dealloc_threshold,
                    usage: usage,
                    properties: properties,
                    is_host_visible: false,
                }
            } else {
                let (buffer, memory) = Buffer::<T>::create_buffer(
                    &device_context,
                    initial_capacity,
                    usage,
                    properties,
                )
                .unwrap();

                let mapped = device
                    .map_memory(
                        memory,
                        0,
                        initial_capacity as u64 * size_of::<T>() as u64,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap()
                    .cast::<T>();
                memcpy(initial_contents.as_ptr(), mapped, initial_contents.len());

                Self {
                    buffer: buffer,
                    memory: memory,
                    mapped: mapped,
                    available_indices: (initial_contents.len() as u32..initial_capacity as u32)
                        .collect(),
                    element_count: initial_contents.len() as u32,
                    element_capacity: initial_capacity as u32,
                    alloc_dealloc_threshold: alloc_dealloc_threshold,
                    usage: usage,
                    properties: properties,
                    is_host_visible: true,
                }
            }
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        unsafe {
            device.device_wait_idle().unwrap();

            if self.mapped != std::ptr::null_mut() {
                device.unmap_memory(self.memory);
                self.mapped = std::ptr::null_mut();
            }

            if !self.buffer.is_null() {
                device.destroy_buffer(self.buffer, None);
                device.free_memory(self.memory, None);
                self.buffer = vk::Buffer::null();
                self.memory = vk::DeviceMemory::null();
            }

            self.available_indices.clear();
        }
    }

    pub fn recreate(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        contents: Vec<T>,
    ) {
        // Destroy old buffer
        self.destroy(&device_context.device);

        // Calculate initial capacity
        let initial_capacity: u32 = (contents.len() as u32)
            .max(
                self.alloc_dealloc_threshold
                    * (contents.len() as f32 / self.alloc_dealloc_threshold as f32).ceil() as u32,
            )
            .max(self.alloc_dealloc_threshold);

        // Create the new buffer
        *self = Buffer::new(
            device_context,
            command_pool,
            initial_capacity as u64,
            self.usage,
            self.properties,
            self.alloc_dealloc_threshold,
            contents,
        );
    }

    pub fn copy(
        &self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        destination: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<()> {
        unsafe {
            let command_buffer =
                begin_single_time_commands(command_pool, device_context.clone().device)?;

            let regions = vk::BufferCopy::builder().size(size);
            device_context.device.cmd_copy_buffer(
                command_buffer,
                self.buffer,
                destination,
                &[regions],
            );

            end_single_time_commands(
                command_pool,
                command_buffer,
                device_context.device.clone(),
                device_context.device_queue_handle.clone(),
            )?;

            Ok(())
        }
    }

    pub fn get_buffer_items(
        &self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        include_empty: bool,
    ) -> Result<Vec<T>> {
        unsafe {
            let mut contents: Vec<T> = Vec::with_capacity(self.element_capacity as usize);
            if self.is_host_visible {
                for i in 0..self.element_capacity {
                    if include_empty || !self.available_indices.contains(&i) {
                        contents.push(self.mapped.add(i as usize).read());
                    }
                }
                return Ok(contents);
            }

            // Create staging buffer to read data from GPU
            let (staging_buffer, staging_memory) = Buffer::<T>::create_buffer(
                &device_context,
                self.element_capacity as u64,
                vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            // Copy buffer contents to staging buffer to make the contents readable
            Buffer::<T>::copy_buffer(
                device_context,
                command_pool,
                self.buffer,
                staging_buffer,
                self.element_capacity as u64,
            )?;

            // Read the data from the buffer
            let memory = device_context
                .device
                .map_memory(
                    staging_memory,
                    0,
                    self.element_capacity as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap()
                .cast::<T>();

            // Create a Vector out of the memory
            let vec: Vec<T> =
                slice::from_raw_parts(memory.cast(), self.element_capacity as usize).to_vec();

            // Cleanup
            device_context.device.destroy_buffer(staging_buffer, None);
            device_context.device.unmap_memory(staging_memory);
            device_context.device.free_memory(staging_memory, None);

            Ok(vec)
        }
    }

    pub fn add_items(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        mut items: Vec<T>,
    ) -> Result<()> {
        while self.is_host_visible
            && items.len() > 0
            && let Some(available_index) = self.available_indices.pop_first()
            && let Some(next_content) = items.pop()
        {
            unsafe { *self.mapped.add(available_index as usize).cast_mut() = next_content };
            self.element_count += 1;
        }

        if items.len() > 0 {
            // Get items from current buffer
            let mut total_items: Vec<T> =
                self.get_buffer_items(device_context, command_pool, false)?;

            // Combine old and new items
            total_items.extend(items);

            self.recreate(device_context, command_pool, total_items);
        }

        Ok(())
    }

    pub fn remove_items(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        start_remove_index: u32,
        stop_remove_index: u32,
    ) -> Result<()> {
        if self.is_host_visible {
            self.element_count -= stop_remove_index - start_remove_index;
            self.available_indices
                .extend(start_remove_index..stop_remove_index);

            // Get furthest element index used
            let mut furthest_used_index: u32 = self.element_capacity - 1;

            if let Some(mut prev_val) = self.available_indices.iter().last() {
                for i in self.available_indices.iter().rev() {
                    if prev_val - i > 1 {
                        break;
                    }
                    prev_val = i;
                }
                furthest_used_index = *prev_val - 1;
            }

            // Get furthest element index not used
            let furthest_empty_index = *self
                .available_indices
                .last()
                .unwrap_or(&(self.element_capacity - 1));

            let num_empty_end_indices = furthest_empty_index - furthest_used_index;

            if num_empty_end_indices >= self.alloc_dealloc_threshold {
                // Get items from current buffer
                let mut total_items: Vec<T> =
                    self.get_buffer_items(device_context, command_pool, true)?;

                for _ in furthest_used_index + 1..=furthest_empty_index {
                    total_items.pop();
                    self.available_indices.pop_last();
                }

                // Create a new buffer with all the items
                let available_indices = self.available_indices.clone();
                self.recreate(device_context, command_pool, total_items);
                self.available_indices = available_indices;
            }
        } else {
            // Get items from current buffer
            let mut total_items: Vec<T> =
                self.get_buffer_items(device_context, command_pool, false)?;

            // Remove in between items
            total_items.drain(start_remove_index as usize..stop_remove_index as usize);

            // Create a new buffer with all the items
            self.recreate(device_context, command_pool, total_items);
        }

        Ok(())
    }

    /// Adds item and returns the index chosen to place the item
    pub fn add_item(
        &mut self,
        context: &DeviceContext,
        command_pool: vk::CommandPool,
        item: T,
    ) -> Result<u32> {
        let available_indices_size = self.available_indices.len();
        let chosen_index = if self.available_indices.len() > 0 {
            self.available_indices.first()
        } else {
            Some(&self.element_count)
        };
        let chosen_index = *chosen_index.unwrap();
        self.add_items(context, command_pool, vec![item])?;
        Ok(chosen_index)
    }

    pub fn remove_item_at(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        index: u32,
    ) -> Result<()> {
        self.remove_items(device_context, command_pool, index, index + 1)?;
        Ok(())
    }

    pub fn create_buffer(
        device_context: &DeviceContext,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        unsafe {
            // Buffer

            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size.max(1) * size_of::<T>() as u64)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let buffer = device_context.device.create_buffer(&buffer_info, None)?;

            // Memory

            let requirements = device_context.device.get_buffer_memory_requirements(buffer);

            let memory_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(requirements.size)
                .memory_type_index(get_memory_type_index(
                    &device_context.instance,
                    device_context.physical_device,
                    properties,
                    requirements,
                )?);

            let buffer_memory = device_context.device.allocate_memory(&memory_info, None)?;

            device_context
                .device
                .bind_buffer_memory(buffer, buffer_memory, 0)?;

            Ok((buffer, buffer_memory))
        }
    }

    pub fn copy_buffer(
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        source: vk::Buffer,
        destination: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<()> {
        unsafe {
            let command_buffer =
                begin_single_time_commands(command_pool, device_context.device.clone())?;

            let regions = vk::BufferCopy::builder().size(size * size_of::<T>() as u64);
            device_context
                .device
                .cmd_copy_buffer(command_buffer, source, destination, &[regions]);

            end_single_time_commands(
                command_pool,
                command_buffer,
                device_context.device.clone(),
                device_context.device_queue_handle.clone(),
            )?;

            Ok(())
        }
    }
}
