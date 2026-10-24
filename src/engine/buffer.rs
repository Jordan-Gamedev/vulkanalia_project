use anyhow::Result;
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
    pub element_count: u32,
    pub element_capacity: u32,
    pub alloc_dealloc_threshold: u32,
    pub usage: vk::BufferUsageFlags,
    pub properties: vk::MemoryPropertyFlags,
    pub is_host_visible: bool,
}

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

            // Pad empty space with default values so that garbage or zeroed out memory isn't used in buffer
            let initial_contents_count = initial_contents.len();
            let mut initial_contents = initial_contents;
            let default_padding_count = initial_capacity as usize - initial_contents_count;
            initial_contents.extend(vec![T::default(); default_padding_count]);

            // If the buffer needs to be initialized with values and it isn't cpu accessible, use staging buffer
            if properties.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
                // Create (staging)

                let (staging_buffer, staging_buffer_memory, mapped) = Buffer::<T>::create_buffer(
                    &device_context,
                    initial_capacity as u64,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                )
                .unwrap();

                // Copy (staging)

                memcpy(initial_contents.as_ptr(), mapped, initial_capacity as usize);
                device.unmap_memory(staging_buffer_memory);

                // Create (device local)

                let usage =
                    vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST | usage;
                let (device_buffer, device_buffer_memory, _) = Buffer::<T>::create_buffer(
                    &device_context,
                    initial_capacity,
                    usage,
                    properties,
                )
                .unwrap();
                if properties.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
                    device.unmap_memory(device_buffer_memory);
                }

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
                    element_count: initial_contents_count as u32,
                    element_capacity: initial_capacity as u32,
                    alloc_dealloc_threshold: alloc_dealloc_threshold,
                    usage: usage,
                    properties: properties,
                    is_host_visible: false,
                }
            } else {
                let (buffer, memory, mapped) = Buffer::<T>::create_buffer(
                    &device_context,
                    initial_capacity,
                    usage,
                    properties,
                )
                .unwrap();

                memcpy(initial_contents.as_ptr(), mapped, initial_capacity as usize);

                Self {
                    buffer: buffer,
                    memory: memory,
                    mapped: mapped,
                    element_count: initial_contents_count as u32,
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
        }
    }

    pub fn recreate(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        contents: Vec<T>,
        descriptor_set: vk::DescriptorSet,
        descriptor_binding: u32,
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
        let new_buffer = Buffer::new(
            device_context,
            command_pool,
            initial_capacity as u64,
            self.usage,
            self.properties,
            self.alloc_dealloc_threshold,
            contents,
        );

        *self = new_buffer;

        self.update_descriptor(&device_context.device, descriptor_set, descriptor_binding);
    }

    pub fn update_descriptor(
        &self,
        device: &Device,
        descriptor_set: vk::DescriptorSet,
        descriptor_binding: u32,
    ) {
        let descriptor_type: vk::DescriptorType =
            if self.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                vk::DescriptorType::STORAGE_BUFFER
            } else {
                vk::DescriptorType::UNIFORM_BUFFER
            };

        let descriptor_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(self.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let descriptor_buffer_info = &[descriptor_buffer_info];

        let descriptor_set_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(descriptor_binding)
            .dst_array_element(0)
            .descriptor_type(descriptor_type)
            .buffer_info(descriptor_buffer_info);

        unsafe {
            device.update_descriptor_sets(&[descriptor_set_write], &[] as &[vk::CopyDescriptorSet]);
        }
    }

    pub fn copy(
        &self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        destination: &mut Buffer<T>,
    ) -> Result<()> {
        unsafe {
            let command_buffer =
                begin_single_time_commands(command_pool, device_context.clone().device)?;

            let regions = vk::BufferCopy::builder()
                .size(self.element_capacity as u64 * size_of::<T>() as u64);
            device_context.device.cmd_copy_buffer(
                command_buffer,
                self.buffer,
                destination.buffer,
                &[regions],
            );

            end_single_time_commands(
                command_pool,
                command_buffer,
                device_context.device.clone(),
                device_context.device_queue_handle.clone(),
            )?;

            destination.element_count = self.element_count;

            Ok(())
        }
    }

    pub fn get_buffer_parts(
        &self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
    ) -> (vk::Buffer, vk::DeviceMemory, *const T) {
        if self.is_host_visible {
            (self.buffer, self.memory, self.mapped)
        } else {
            // Create staging buffer to read data from GPU
            let (staging_buffer, staging_memory, mapped) = Buffer::<T>::create_buffer(
                &device_context,
                self.element_capacity as u64,
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )
            .unwrap();

            // Copy buffer contents to staging buffer to make the contents readable
            Buffer::<T>::copy_buffer(
                device_context,
                command_pool,
                self.buffer,
                staging_buffer,
                self.element_capacity as u64,
            )
            .unwrap();

            // Read the data from the buffer
            let mapped = mapped.cast_const();

            (staging_buffer, staging_memory, mapped)
        }
    }

    pub fn get_buffer_items(
        &self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        include_empty: bool,
    ) -> Result<Vec<T>> {
        // Return empty vector if there are no elements in buffer and empty items are not desired
        if !include_empty && self.element_count == 0 {
            return Ok(Vec::new());
        }

        unsafe {
            // Get CPU visible buffer data
            let (buffer, memory, mapped) = self.get_buffer_parts(device_context, command_pool);

            // Collect contents
            let mut contents: Vec<T> = Vec::with_capacity(self.element_capacity as usize);
            for i in 0..self.element_capacity {
                if include_empty || i < self.element_count {
                    contents.push(mapped.add(i as usize).read());
                }
            }

            // Cleanup
            if !self.is_host_visible {
                device_context.device.destroy_buffer(buffer, None);
                device_context.device.unmap_memory(memory);
                device_context.device.free_memory(memory, None);
            }

            Ok(contents)
        }
    }

    pub fn add_items(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        descriptor_set: vk::DescriptorSet,
        descriptor_binding: u32,
        mut items: Vec<T>,
    ) -> Result<()> {
        let (buffer, memory, mapped) = self.get_buffer_parts(device_context, command_pool);

        if items.len() as u32 + self.element_count > self.element_capacity {
            // Get items from current buffer
            let mut total_items: Vec<T> = unsafe {
                std::slice::from_raw_parts(mapped.cast(), self.element_capacity as usize).to_vec()
            };

            // Combine old and new items
            total_items.extend(items);

            self.recreate(
                device_context,
                command_pool,
                total_items,
                descriptor_set,
                descriptor_binding,
            );
        } else {
            while items.len() > 0 {
                unsafe { *mapped.add(self.element_count as usize).cast_mut() = items.remove(0) };
                self.element_count += 1;
            }

            if !self.is_host_visible {
                Buffer::<T>::copy_buffer(
                    &device_context,
                    command_pool,
                    buffer,
                    self.buffer,
                    self.element_capacity as u64,
                )
                .unwrap();
            }
        }

        // Cleanup
        unsafe {
            if !self.is_host_visible {
                device_context.device.destroy_buffer(buffer, None);
                device_context.device.unmap_memory(memory);
                device_context.device.free_memory(memory, None);
            }
        }

        Ok(())
    }

    pub fn remove_items(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        descriptor_set: vk::DescriptorSet,
        descriptor_binding: u32,
        start_remove_index: u32,
        stop_remove_index: u32,
    ) -> Result<()> {
        // Get items from current buffer
        let mut total_items: Vec<T> = self.get_buffer_items(device_context, command_pool, false)?;

        // Remove in between items
        total_items.drain(start_remove_index as usize..stop_remove_index as usize);

        // Create a new buffer with all the items
        self.recreate(
            device_context,
            command_pool,
            total_items,
            descriptor_set,
            descriptor_binding,
        );

        Ok(())
    }

    /// Adds item and returns the index chosen to place the item
    pub fn add_item(
        &mut self,
        context: &DeviceContext,
        command_pool: vk::CommandPool,
        descriptor_set: vk::DescriptorSet,
        descriptor_binding: u32,
        item: T,
    ) -> Result<u32> {
        let chosen_index = self.element_count;
        self.add_items(
            context,
            command_pool,
            descriptor_set,
            descriptor_binding,
            vec![item],
        )?;
        Ok(chosen_index)
    }

    pub fn remove_item_at(
        &mut self,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        descriptor_set: vk::DescriptorSet,
        descriptor_binding: u32,
        index: u32,
    ) -> Result<()> {
        self.remove_items(
            device_context,
            command_pool,
            descriptor_set,
            descriptor_binding,
            index,
            index + 1,
        )?;
        Ok(())
    }

    pub fn create_buffer(
        device_context: &DeviceContext,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory, *mut T)> {
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

            let mapped = if properties.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
                device_context
                    .device
                    .map_memory(buffer_memory, 0, size.max(1), vk::MemoryMapFlags::empty())
                    .unwrap()
                    .cast::<T>()
            } else {
                std::ptr::null_mut()
            };

            Ok((buffer, buffer_memory, mapped))
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
