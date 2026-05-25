#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::{anyhow, Result};
use vulkanalia::prelude::v1_0::*;
use std::{collections::HashMap, mem::size_of};
use std::ptr::copy_nonoverlapping as memcpy;
use glam::{vec2, vec3, Vec2, Vec3, Mat4};

use crate::engine::{CommandEngine};

use super::device_context::DeviceContext;

#[derive(Clone, Default)]
pub struct ModelEngine {
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    pub loaded_models: HashMap<String, Model>,
}

impl ModelEngine {
    pub fn destroy(&mut self, device: Device) {
        unsafe {
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
            self.uniform_buffers.iter().for_each(|b| device.destroy_buffer(*b, None));
            self.uniform_buffers_memory.iter().for_each(|m| device.free_memory(*m, None));
        }
    }

    /// Todo: make it actually use the path (or some other method) to get the data
    pub fn load_model(&mut self, context: DeviceContext, command_engine: CommandEngine, path: String) -> Result<()> {
        // Add one to the instance count if the model is already loaded and early exit
        if let Some(model) = self.loaded_models.get_mut(&path) {
            model.instance_count += 1;
            return Ok(())
        }
        
        // Get vertices
    
        let vertex_bytes: &[u8; _] = include_bytes!("../../assets/models_compressed/Limpet.vertbuff");
        let vertex_count = u32::from_be_bytes(vertex_bytes[0..size_of::<u32>()].try_into().unwrap()) as usize;
    
        let vertices: Vec<QuantizedVertex> = match meshopt::decode_vertex_buffer(vertex_bytes[size_of::<u32>()..].try_into().unwrap(), vertex_count) {
            Ok(bytes) => bytes,
            Err(_) => return Err(anyhow!("Failed to decode vertex buffer")),
        };
    
        // Get indices
    
        let index_bytes: &[u8; _] = include_bytes!("../../assets/models_compressed/Limpet.indbuff");
        let index_count = u32::from_be_bytes(index_bytes[0..size_of::<u32>()].try_into().unwrap()) as usize;
        let indices: Vec<u32> = match meshopt::decode_index_buffer(index_bytes[size_of::<u32>()..].try_into().unwrap(), index_count) {
            Ok(indices) => indices,
            Err(_) => return Err(anyhow!("Failed to decode index buffer")),
        };

        unsafe {
            self.add_vertex_buffer(context.clone(), &command_engine, vertices)?;
            self.add_index_buffer(context, &command_engine, indices)?;
        }

        let model = Model {
            vertex_offset: self.get_vertex_count() as u32,
            vertex_length: vertex_count as u32,
            index_offset: self.get_index_count() as u32,
            index_length: index_count as u32,
            instance_count: 1,
        };
        self.loaded_models.insert(path.clone(), model);

        println!("Successfully loaded model: {}", path);

        Ok(())
    }

    pub fn unload_model(&mut self, context: DeviceContext, command_engine: CommandEngine, path: String) -> Result<()> {
        let (unloading_model, fully_unloaded) = if let Some(model) = self.loaded_models.get_mut(&path) {
            model.instance_count -= 1;
            (*model, model.instance_count == 0)
        } else {
            return Err(anyhow!("Model not found"))
        };
        
        if fully_unloaded {
            // Unload model from memory
            self.loaded_models.remove(&path);
            unsafe {
                self.remove_vertex_buffer(context.clone(), &command_engine, unloading_model)?;
                self.remove_index_buffer(context, &command_engine, unloading_model)?;
            }

            // Update other model offsets
            self.loaded_models.values_mut()
                .filter(|m| m.vertex_offset > unloading_model.vertex_offset)
                .for_each(|m| {
                    m.vertex_offset -= unloading_model.vertex_length;
                    m.index_offset -= unloading_model.index_length;
                });
        }
        
        Ok(())
    }

    pub fn get_supported_vertex_format(
        context: &DeviceContext,
        candidates: &[vk::Format],
        features: vk::FormatFeatureFlags,
    ) -> Result<vk::Format> {
        candidates
            .iter()
            .cloned()
            .find(|f| {
                let properties = unsafe { context.instance.get_physical_device_format_properties(context.physical_device, *f) };
                // For vertex buffers, check buffer features (typically linear tiling)
                properties.buffer_features.contains(features)
            })
            .ok_or_else(|| anyhow!("Failed to find supported vertex attribute format"))
    }

    pub fn get_vertex_count(&self) -> u64 {
        let mut count: u64 = 0;
        for model in self.loaded_models.values() {
            count += model.vertex_length as u64;
        }
        count
    }

    pub fn get_index_count(&self) -> u64 {
        let mut count: u64 = 0;
        for model in self.loaded_models.values() {
            count += model.index_length as u64;
        }
        count
    }
    
    pub fn get_buffer_contents<T>(context: DeviceContext, command_engine: &CommandEngine, buffer: vk::Buffer, content_length: usize) -> Result<Vec<T>> {
        unsafe {
            // Create staging buffer to read data from GPU
            let size: usize = content_length * size_of::<T>();
            let (staging_buffer, staging_memory) = ModelEngine::create_buffer(
                context.clone(),
                size as u64,
                vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            // Copy buffer contents to staging buffer to make the contents readable
            ModelEngine::copy_buffer(context.clone(), command_engine, buffer, staging_buffer, size as u64)?;

            // Read the data from the buffer
            let memory = context.device.map_memory(
                staging_memory,
                0,
                size as u64,
                vk::MemoryMapFlags::empty()
            )
            .unwrap()
            .cast::<T>();

            // Create a Vector out of the memory
            let vec: Vec<T> = Vec::from_raw_parts(memory, size as usize, size as usize);

            // Cleanup
            context.device.destroy_buffer(staging_buffer, None);
            context.device.unmap_memory(staging_memory);
            context.device.free_memory(staging_memory, None);

            Ok(vec)
        }
    }

    unsafe fn create_vertex_buffer(&mut self, context: DeviceContext, command_engine: &CommandEngine, vertices: Vec<QuantizedVertex>) -> Result<()> {
        let device = context.clone().device;

        // Destroy old vertex buffer

        if !self.vertex_buffer.is_null() {
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
        }

        // Set the buffer to null if there are no vertices to make the buffer out of

        if vertices.len() == 0 {
            self.vertex_buffer = vk::Buffer::null();
            return Ok(())
        }

        // Create (staging)
    
        let size = (size_of::<QuantizedVertex>() * vertices.len()) as u64;
    
        let (staging_buffer, staging_buffer_memory) = ModelEngine::create_buffer(
            context.clone(),
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
    
        // Copy (staging)
    
        let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;
        memcpy(vertices.as_ptr(), memory.cast(), vertices.len());
        device.unmap_memory(staging_buffer_memory);
    
        // Create (vertex)
    
        let (vertex_buffer, vertex_buffer_memory) = ModelEngine::create_buffer(
            context.clone(),
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.vertex_buffer = vertex_buffer;
        self.vertex_buffer_memory = vertex_buffer_memory;
    
        // Copy (vertex)
    
        ModelEngine::copy_buffer(context, command_engine, staging_buffer, vertex_buffer, size)?;
    
        // Cleanup
    
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    
        Ok(())
    }
    
    unsafe fn create_index_buffer(&mut self, context: DeviceContext, command_engine: &CommandEngine, indices: Vec<u32>) -> Result<()> {
        let device = context.clone().device;

        // Destroy old index buffer

        if !self.index_buffer.is_null() {
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
        }

        // Set the buffer to null if there are no indices to make the buffer out of

        if indices.len() == 0 {
            self.index_buffer = vk::Buffer::null();
            return Ok(())
        }

        // Create (staging)
    
        let size = (size_of::<u32>() * indices.len()) as u64;
    
        let (staging_buffer, staging_buffer_memory) = ModelEngine::create_buffer(
            context.clone(),
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
    
        // Copy (staging)
    
        let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;
        memcpy(indices.as_ptr(), memory.cast(), indices.len());
        device.unmap_memory(staging_buffer_memory);
    
        // Create (index)
    
        let (index_buffer, index_buffer_memory) = ModelEngine::create_buffer(
            context.clone(),
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        self.index_buffer = index_buffer;
        self.index_buffer_memory = index_buffer_memory;
    
        // Copy (index)
    
        ModelEngine::copy_buffer(context, command_engine, staging_buffer, self.index_buffer, size)?;
    
        // Cleanup
    
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_buffer_memory, None);
    
        Ok(())
    }
    
    unsafe fn add_vertex_buffer(&mut self, context: DeviceContext, command_engine: &CommandEngine, vertices: Vec<QuantizedVertex>) -> Result<()> {
        // Create a vertex buffer for the vertices if none was yet made
        if self.vertex_buffer.is_null() {
            return self.create_vertex_buffer(context, command_engine, vertices)
        }
        
        // Get vertices from current vertex buffer
        let mut total_vertices = ModelEngine::get_buffer_contents::<QuantizedVertex>(context.clone(), command_engine, self.vertex_buffer, self.get_vertex_count() as usize)?;

        // Combine old and new vertices
        total_vertices.extend(vertices);

        // Create a new vertex buffer with all the vertices
        self.create_vertex_buffer(context.clone(), command_engine, total_vertices)?;

        Ok(())
    }

    unsafe fn add_index_buffer(&mut self, context: DeviceContext, command_engine: &CommandEngine, indices: Vec<u32>) -> Result<()> {
        // Create an index buffer for the indices if none was yet made
        if self.index_buffer.is_null() {
            return self.create_index_buffer(context, command_engine, indices)
        }
        
        // Get indices from current index buffer
        let mut total_indices = ModelEngine::get_buffer_contents::<u32>(context.clone(), command_engine, self.vertex_buffer, self.get_index_count() as usize)?;

        // Combine old and new indices
        total_indices.extend(indices);

        // Create a new index buffer with all the indices
        self.create_index_buffer(context.clone(), command_engine, total_indices)?;

        Ok(())
    }

    unsafe fn remove_vertex_buffer(&mut self, context: DeviceContext, command_engine: &CommandEngine, model: Model) -> Result<()> {        
        // Get vertices from current vertex buffer
        let mut vertices = ModelEngine::get_buffer_contents::<QuantizedVertex>(context.clone(), command_engine, self.vertex_buffer, self.get_vertex_count() as usize)?;
        
        // Remove designated vertices
        vertices.drain((model.vertex_offset as usize)..((model.vertex_offset + model.vertex_length) as usize));

        // Create a new vertex buffer with the remaining vertices
        self.create_vertex_buffer(context, command_engine, vertices)?;

        Ok(())
    }

    unsafe fn remove_index_buffer(&mut self, context: DeviceContext, command_engine: &CommandEngine, model: Model) -> Result<()> {        
        // Get indices from current index buffer
        let mut indices = ModelEngine::get_buffer_contents::<u32>(context.clone(), command_engine, self.vertex_buffer, self.get_index_count() as usize)?;
        
        // Remove designated indices
        indices.drain((model.index_offset as usize)..((model.index_offset + model.index_length) as usize));

        // Create a new index buffer with the remaining indices
        self.create_index_buffer(context, command_engine, indices)?;

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
                .size(size)
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
    
            let regions = vk::BufferCopy::builder().size(size);
            context.device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);
        
            command_engine.end_single_time_commands(context, command_buffer)?;
        
            Ok(())    
        }
    }
}

pub struct ModelEngineBuilder(pub(crate) ModelEngine);

impl ModelEngineBuilder {
    pub fn new() -> Self {
        Self(ModelEngine::default())
    }

    pub unsafe fn create_uniform_buffers(&mut self, context: DeviceContext, command_engine: CommandEngine) -> Result<()> {
        self.0.uniform_buffers.iter().for_each(|b| context.device.destroy_buffer(*b, None));
        self.0.uniform_buffers_memory.iter().for_each(|m| context.device.free_memory(*m, None));
        self.0.uniform_buffers.clear();
        self.0.uniform_buffers_memory.clear();

        for _ in 0..command_engine.max_frames_in_flight {
            let (uniform_buffer, uniform_buffer_memory) = ModelEngine::create_buffer(
                context.clone(),
                size_of::<UniformBufferObject>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            )?;
    
            self.0.uniform_buffers.push(uniform_buffer);
            self.0.uniform_buffers_memory.push(uniform_buffer_memory);
        }
    
        Ok(())
    }

    pub unsafe fn create_model_matrix_buffer(&mut self, context: DeviceContext) -> Result<()> {

        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    pub pos: Vec3,
    pub color: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

impl Vertex {
    pub const fn new(pos: Vec3, color: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self { pos, color, normal, uv }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();

        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset((size_of::<Vec3>() * 2) as u32)
            .build();

        let uv = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32_SFLOAT)
            .offset((size_of::<Vec3>() * 3) as u32)
            .build();

        [pos, color, normal, uv]
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct QuantizedVertex {
    pub position: [u16; 3],
    pub color: [u8; 3],
    pub normal: [i8; 3],
    pub uv: [u16; 2],
}

impl QuantizedVertex {
    pub const fn from_slice(slice: &[u8; 16]) -> Self {
        let position = [u16::from_le_bytes([slice[0], slice[1]]), u16::from_le_bytes([slice[2], slice[3]]), u16::from_le_bytes([slice[4], slice[5]])];
        let color = [slice[6], slice[7], slice[8]];
        let normal = [slice[9] as i8, slice[10] as i8, slice[11] as i8];
        let uv = [u16::from_le_bytes([slice[12], slice[13]]), u16::from_le_bytes([slice[14], slice[15]])];

        Self { position, color, normal, uv }
    }

    pub fn to_vertex(&self) -> Vertex {
        let position: Vec3 = vec3(
            meshopt::dequantize_half(self.position[0]),
            meshopt::dequantize_half(self.position[1]),
            meshopt::dequantize_half(self.position[2]),
        );

        let color: Vec3 = vec3(
            self.color[0] as f32 / u8::MAX as f32,
            self.color[1] as f32 / u8::MAX as f32,
            self.color[2] as f32 / u8::MAX as f32,
        );

        let normal: Vec3 = vec3(
            self.normal[0] as f32 / i8::MAX as f32,
            self.normal[1] as f32 / i8::MAX as f32,
            self.normal[2] as f32 / i8::MAX as f32,
        );

        let uv: Vec2 = vec2(
            self.uv[0] as f32 / u16::MAX as f32,
            self.uv[1] as f32 / u16::MAX as f32,
        );

        Vertex {
            pos: position,
            color: color,
            normal: normal,
            uv: uv,
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<QuantizedVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions(context: &DeviceContext) -> Result<[vk::VertexInputAttributeDescription; 4]> {
        // Try preferred format, fall back if not supported
        // Vertex attributes typically support FORMAT_VERTEX_BUFFER feature
        let features = vk::FormatFeatureFlags::VERTEX_BUFFER;
        
        // Position: Try R16G16B16_SFLOAT, fall back to R16G16B16A16_SFLOAT
        let pos_format = ModelEngine::get_supported_vertex_format(
            context,
            &[vk::Format::R16G16B16_SFLOAT, vk::Format::R16G16B16A16_SFLOAT],
            features,
        )?;
        let pos = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(pos_format)
            .offset(0)
            .build();

        // Color: Try R8G8B8_UNORM, fall back to R8G8B8A8_UNORM
        let color_format = ModelEngine::get_supported_vertex_format(
            context,
            &[vk::Format::R8G8B8_UNORM, vk::Format::R8G8B8A8_UNORM],
            features,
        )?;
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(color_format)
            .offset(size_of::<[u16; 3]>() as u32)
            .build();

        // Normal: Try R8G8B8_SNORM, fall back to R8G8B8A8_SNORM
        let normal_format = ModelEngine::get_supported_vertex_format(
            context,
            &[vk::Format::R8G8B8_SNORM, vk::Format::R8G8B8A8_SNORM],
            features,
        )?;
        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(normal_format)
            .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>()) as u32)
            .build();
        
        // UV: Use R16G16_UNORM
        let uv = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R16G16_UNORM)
            .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>() + size_of::<[i8; 3]>()) as u32)
            .build();

        Ok([pos, color, normal, uv])
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Model {
    pub vertex_offset: u32,
    pub vertex_length: u32,
    pub index_offset: u32,
    pub index_length: u32,
    pub instance_count: u32,
}