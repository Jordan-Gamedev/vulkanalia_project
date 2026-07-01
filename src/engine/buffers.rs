#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::Result;
use vulkanalia::prelude::v1_0::*;
use core::slice;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;

use crate::engine::CommandEngine;

use super::device_context::DeviceContext;

#[derive(Clone, Default)]
pub struct Buffer<T: Clone + Default> {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub mapped: *mut T,
    pub element_count: u32,
    pub element_capacity: u32,
    pub alloc_dealloc_threshold: u32,
    pub usage: vk::BufferUsageFlags,
    pub properties: vk::MemoryPropertyFlags,
    pub is_host_visible: bool,
}

impl<T: Clone + Default> Buffer<T> {
    pub fn new(
        context: DeviceContext,
        command_engine: &CommandEngine,
        initial_capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        alloc_dealloc_threshold: u32,
        initial_contents: Vec<T>,
    ) -> Self {
        unsafe {
            let device = context.clone().device;
    
            // If the buffer needs to be initialized with values and it isn't cpu accessible, use staging buffer
            if properties.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL) {
                // Create (staging)
    
                let (staging_buffer, staging_buffer_memory) = Buffer::<T>::create_buffer(
                    context.clone(),
                    initial_capacity as u64,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
                ).unwrap();
            
                // Copy (staging)
            
                let mapped = device.map_memory(staging_buffer_memory, 0, initial_capacity, vk::MemoryMapFlags::empty()).unwrap().cast();
                memcpy(initial_contents.as_ptr(), mapped, initial_contents.len());
                device.unmap_memory(staging_buffer_memory);

                // Create (device local)
    
                let usage = vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST | usage;
                let (device_buffer, device_buffer_memory) = Buffer::<T>::create_buffer(
                    context.clone(),
                    initial_capacity,
                    usage,
                    properties,
                ).unwrap();

                // Copy (device local)
            
                Buffer::<T>::copy_buffer(context, command_engine, staging_buffer, device_buffer, initial_capacity).unwrap();

                device.destroy_buffer(staging_buffer, None);
                device.free_memory(staging_buffer_memory, None);

                Self {
                    buffer: device_buffer,
                    memory: device_buffer_memory,
                    mapped: std::ptr::null_mut(),
                    element_count: initial_contents.len() as u32,
                    element_capacity: initial_capacity as u32,
                    alloc_dealloc_threshold: alloc_dealloc_threshold,
                    usage: usage,
                    properties: properties,
                    is_host_visible: false,
                }
            } else {
                let (buffer, memory) = Buffer::<T>::create_buffer(context, initial_capacity, usage, properties).unwrap();
                let mapped = device.map_memory(memory, 0, initial_capacity as u64 * size_of::<T>() as u64, vk::MemoryMapFlags::empty()).unwrap().cast::<T>();
                memcpy(initial_contents.as_ptr(), mapped, initial_contents.len());

                Self {
                    buffer: buffer,
                    memory: memory,
                    mapped: mapped,
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

    pub fn destroy(&mut self, device: Device) {
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

    pub fn recreate(&mut self, context: DeviceContext, command_engine: &CommandEngine, contents: Vec<T>) {
        // Destroy old buffer
        self.destroy(context.clone().device);

        // Calculate initial capacity
        let initial_capacity: u32 = (contents.len() as u32).max(self.alloc_dealloc_threshold * (contents.len() as f32 / self.alloc_dealloc_threshold as f32).ceil() as u32);

        // Create the new buffer
        *self = Buffer::new(
            context,
            command_engine,
            initial_capacity as u64,
            self.usage,
            self.properties,
            self.alloc_dealloc_threshold,
            contents
        );
    }
    
    pub fn copy(
        &self,
        context: DeviceContext,
        command_engine: &CommandEngine,
        destination: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<()> {
        unsafe {
            let command_buffer = command_engine.begin_single_time_commands(context.clone().device)?;
    
            let regions = vk::BufferCopy::builder().size(size);
            context.device.cmd_copy_buffer(command_buffer, self.buffer, destination, &[regions]);
        
            command_engine.end_single_time_commands(context, command_buffer)?;
        
            Ok(())    
        }
    }

    pub fn get_buffer_contents(&self, context: DeviceContext, command_engine: &CommandEngine, include_empty: bool) -> Result<Vec<T>> {
        unsafe {
            let end = if include_empty { self.element_capacity } else { self.element_count };

            let mut contents: Vec<T> = Vec::with_capacity(self.element_capacity as usize);
            if self.is_host_visible {
                for i in 0..end {
                    contents.push(self.mapped.add(i as usize).read());
                }
                return Ok(contents)
            }

            // Create staging buffer to read data from GPU
            let (staging_buffer, staging_memory) = Buffer::<T>::create_buffer(
                context.clone(),
                self.element_capacity as u64,
                vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            // Copy buffer contents to staging buffer to make the contents readable
            Buffer::<T>::copy_buffer(context.clone(), command_engine, self.buffer, staging_buffer, self.element_capacity as u64)?;

            // Read the data from the buffer
            let memory = context.device.map_memory(
                staging_memory,
                0,
                self.element_capacity as u64,
                vk::MemoryMapFlags::empty()
            )
            .unwrap()
            .cast::<T>();

            // Create a Vector out of the memory
            let vec: Vec<T> = slice::from_raw_parts(memory.cast(), end as usize).to_vec();

            // Cleanup
            context.device.destroy_buffer(staging_buffer, None);
            context.device.unmap_memory(staging_memory);
            context.device.free_memory(staging_memory, None);

            Ok(vec)
        }
    }

    pub fn add_contents(&mut self, context: DeviceContext, command_engine: &CommandEngine, contents: Vec<T>) -> Result<()> {
        // Create a buffer for the content if none was yet made
        if self.buffer.is_null() {
            self.recreate(context, command_engine, contents);
            return Ok(())
        }
        
        let new_content_count = contents.len() as u32;

        if self.is_host_visible && self.element_count + new_content_count <= self.element_capacity {
            // Write changes without recreating the buffer
            unsafe { memcpy(contents.as_ptr(), self.mapped.add(self.element_count as usize), contents.len()); }
            self.element_count += new_content_count;
        } else {
            // Get content from current buffer
            let mut total_contents: Vec<T> = self.get_buffer_contents(context.clone(), command_engine, false)?;
    
            // Combine old and new contents
            total_contents.extend(contents);
            self.recreate(context, command_engine, total_contents);
        }

        Ok(())
    }

    pub fn remove_contents(&mut self, context: DeviceContext, command_engine: &CommandEngine, start_remove_index: u32, stop_remove_index: u32) -> Result<()> {
        // Create a buffer for the content if none was yet made
        if self.buffer.is_null() {
            return Ok(())
        }
        
        // Get content from current buffer
        let mut total_contents: Vec<T> = self.get_buffer_contents(context.clone(), command_engine, false)?;

        // Combine old and new contents
        total_contents.drain(start_remove_index as usize..stop_remove_index as usize);

        if !self.is_host_visible || self.element_count <= self.element_capacity - self.alloc_dealloc_threshold {
            // Create a new buffer with all the contents
            self.recreate(context, command_engine, total_contents);
        }
        else {
            // Write changes without recreating the buffer
            unsafe {
                for i in (total_contents.len() - 1)..self.element_capacity as usize {
                    *self.mapped.add(i) = T::default();
                }
                memcpy(total_contents.as_ptr(), self.mapped, total_contents.len());
                self.element_count -= stop_remove_index - start_remove_index;
            }
        }

        Ok(())
    }

    pub fn create_buffer(
        context: DeviceContext,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        unsafe {
            // Buffer
        
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size * size_of::<T>() as u64)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);
        
            let buffer = context.device.create_buffer(&buffer_info, None)?;
        
            // Memory
        
            let requirements = context.device.get_buffer_memory_requirements(buffer);
        
            let memory_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(requirements.size)
                .memory_type_index(context.get_memory_type_index(properties, requirements)?);
        
            let buffer_memory = context.device.allocate_memory(&memory_info, None)?;
        
            context.device.bind_buffer_memory(buffer, buffer_memory, 0)?;
        
            Ok((buffer, buffer_memory))
        }
    }

    pub fn copy_buffer(
        context: DeviceContext,
        command_engine: &CommandEngine,
        source: vk::Buffer,
        destination: vk::Buffer,
        size: vk::DeviceSize,
    ) -> Result<()> {
        unsafe {
            let command_buffer = command_engine.begin_single_time_commands(context.clone().device)?;
    
            let regions = vk::BufferCopy::builder().size(size * size_of::<T>() as u64);
            context.device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);
        
            command_engine.end_single_time_commands(context, command_buffer)?;
        
            Ok(())    
        }
    }
}