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
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use glam::{Mat4, Quat, Vec2, Vec3, vec2, vec3};

use crate::engine::{CommandEngine};

use super::device_context::DeviceContext;

// Allocates/Deallocates 32KiB at a time
const MODEL_MATRIX_ALLOCATE_THRESHOLD: u32 = 1024;

#[derive(Clone, Default)]
pub struct ModelEngine {
    // Model Shape
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    
    // Camera
    pub uniform_buffers: Vec<vk::Buffer>,
    pub uniform_buffers_memory: Vec<vk::DeviceMemory>,
    
    // Dynamic Transforms
    pub dyn_model_matrix_buffer: vk::Buffer,
    pub dyn_model_matrix_buffer_memory: vk::DeviceMemory,
    pub dyn_model_matrix_buffer_mapped: *mut c_void,
    pub dyn_active_model_matrix_count: u32,
    pub dyn_available_model_matrix_indices: Vec<u32>,
    pub dyn_model_matrices_buffer_contents: Vec<QuantizedModelMatrix>,

    // Static Transforms (TODO: DO NOT STORE MATRICES IN CPU MEMORY, SINCE THERE IS NO BENEFIT YET TO USING STATIC)
    pub static_model_matrix_buffer: vk::Buffer,
    pub static_model_matrix_buffer_memory: vk::DeviceMemory,
    pub static_active_model_matrix_count: u32,
    pub static_available_model_matrix_indices: Vec<u32>,
    pub static_model_matrices_buffer_contents: Vec<QuantizedModelMatrix>,

    // Actively loaded models
    pub loaded_models: HashMap<String, Model>,
}

impl ModelEngine {
    pub fn destroy(&mut self, device: Device) {
        unsafe {
            // Destroy vertex and index
            device.destroy_buffer(self.vertex_buffer, None);
            device.free_memory(self.vertex_buffer_memory, None);
            device.destroy_buffer(self.index_buffer, None);
            device.free_memory(self.index_buffer_memory, None);
            // Destroy ubo
            self.uniform_buffers.iter().for_each(|b| device.destroy_buffer(*b, None));
            self.uniform_buffers_memory.iter().for_each(|m| device.free_memory(*m, None));
            // Destroy ssbos
            device.destroy_buffer(self.dyn_model_matrix_buffer, None);
            device.destroy_buffer(self.static_model_matrix_buffer, None);
            device.unmap_memory(self.dyn_model_matrix_buffer_memory);
            device.free_memory(self.dyn_model_matrix_buffer_memory, None);
            device.free_memory(self.static_model_matrix_buffer_memory, None);
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

    pub fn create_model_matrix(&mut self, context: DeviceContext, is_static: bool) -> Result<u32> {
        if is_static {
            if let Some(available_index) = self.static_available_model_matrix_indices.pop() {
                self.static_model_matrices_buffer_contents[available_index as usize];
                self.static_active_model_matrix_count += 1;
                return Ok(available_index)
            } else {
                // Increase allocation if the threshold is already reached
                if self.static_active_model_matrix_count % MODEL_MATRIX_ALLOCATE_THRESHOLD == 0 {
                    unsafe { self.create_static_model_matrix_buffer(context, self.static_active_model_matrix_count as u64 + MODEL_MATRIX_ALLOCATE_THRESHOLD as u64)?; }
                }
                self.static_active_model_matrix_count += 1;
                return Ok(self.static_active_model_matrix_count - 1)
            }    
        } else {
            if let Some(available_index) = self.dyn_available_model_matrix_indices.pop() {
                self.dyn_model_matrices_buffer_contents[available_index as usize];
                self.dyn_active_model_matrix_count += 1;
                return Ok(available_index)
            } else {
                // Increase allocation if the threshold is already reached
                if self.dyn_active_model_matrix_count % MODEL_MATRIX_ALLOCATE_THRESHOLD == 0 {
                    unsafe { self.create_dyn_model_matrix_buffer(context, self.dyn_active_model_matrix_count as u64 + MODEL_MATRIX_ALLOCATE_THRESHOLD as u64)?; }
                }
                self.dyn_active_model_matrix_count += 1;
                return Ok(self.dyn_active_model_matrix_count - 1)
            }    
        }        
    }

    pub fn remove_model_matrix(&mut self, context: DeviceContext, model_matrix_index: u32, is_static: bool) -> Result<()> {
        if is_static { 
            self.static_model_matrices_buffer_contents[model_matrix_index as usize] = QuantizedModelMatrix::default();
            self.static_available_model_matrix_indices.push(model_matrix_index);
            self.static_active_model_matrix_count -= 1;
            
            // Deallocate if there is too much unused allocation at the end of the buffer
            let max_available_index = *self.static_available_model_matrix_indices.iter().max().unwrap() as usize;
            if self.static_active_model_matrix_count > 0 && self.static_model_matrices_buffer_contents.len() - 1 - max_available_index >= MODEL_MATRIX_ALLOCATE_THRESHOLD as usize {
                self.static_model_matrices_buffer_contents.truncate(self.static_model_matrices_buffer_contents.len() - MODEL_MATRIX_ALLOCATE_THRESHOLD as usize);
                unsafe { self.create_static_model_matrix_buffer(context, self.static_model_matrices_buffer_contents.len() as u64)? };
            }
        } else {
            self.dyn_model_matrices_buffer_contents[model_matrix_index as usize] = QuantizedModelMatrix::default();
            self.dyn_available_model_matrix_indices.push(model_matrix_index);
            self.dyn_active_model_matrix_count -= 1;
            
            // Deallocate if there is too much unused allocation at the end of the buffer
            let max_available_index = *self.dyn_available_model_matrix_indices.iter().max().unwrap() as usize;
            if self.dyn_active_model_matrix_count > 0 && self.dyn_model_matrices_buffer_contents.len() - 1 - max_available_index >= MODEL_MATRIX_ALLOCATE_THRESHOLD as usize {
                self.dyn_model_matrices_buffer_contents.truncate(self.dyn_model_matrices_buffer_contents.len() - MODEL_MATRIX_ALLOCATE_THRESHOLD as usize);
                unsafe { self.create_dyn_model_matrix_buffer(context, self.dyn_model_matrices_buffer_contents.len() as u64)? };
            }
        }
        
        Ok(())
    }

    pub fn update_model_matrix(&mut self, model_matrix_index: u32, position: Vec3, rotation: Quat, scale: Vec3, is_static: bool) -> Result<()> {
        let buffer_contents = if is_static { &mut self.static_model_matrices_buffer_contents } else { &mut self.dyn_model_matrices_buffer_contents };
        
        if let Some(model_matrix) = buffer_contents.get_mut(model_matrix_index as usize) {
            model_matrix.position = position.to_array();
            model_matrix.scale = scale.to_array();
            let rotation_u16: [u16; 4] = [
                (rotation.x * 65535.0) as u16,
                (rotation.y * 65535.0) as u16,
                (rotation.z * 65535.0) as u16,
                (rotation.w * 65535.0) as u16,
            ];
            model_matrix.rotation = rotation_u16;
            Ok(())
        } else {
            Err(anyhow!("Error: Failed to update model matrix (index out of bounds)"))
        }
    }

    pub fn transfer_cpu_model_matrices(&mut self, device: Device) {
        unsafe {
            if self.dyn_model_matrices_buffer_contents.len() > 0 {
                memcpy(self.dyn_model_matrices_buffer_contents.as_ptr(), self.dyn_model_matrix_buffer_mapped.cast(), self.dyn_model_matrices_buffer_contents.len());
            }
    
            if self.static_model_matrices_buffer_contents.len() > 0 {
                //let memory = device.map_memory(memory, offset, size, flags)
                //memcpy(self.static_model_matrices_buffer_contents.as_ptr(), self.static_model_matrix_buffer_mapped.cast(), self.static_model_matrices_buffer_contents.len());
            }    
        }
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

    unsafe fn create_dyn_model_matrix_buffer(&mut self, context: DeviceContext, size: vk::DeviceSize) -> Result<()> {
        if !self.dyn_model_matrix_buffer.is_null() {
            context.device.destroy_buffer(self.dyn_model_matrix_buffer, None);
        }
        
        if !self.dyn_model_matrix_buffer_memory.is_null() {
            context.device.unmap_memory(self.dyn_model_matrix_buffer_memory);
            context.device.free_memory(self.dyn_model_matrix_buffer_memory, None);
            self.dyn_model_matrix_buffer_mapped = std::ptr::null_mut();
        }

        self.dyn_model_matrices_buffer_contents.resize(size as usize, QuantizedModelMatrix::default());

        let (ssbo, ssbo_memory) = ModelEngine::create_buffer(
            context.clone(),
            size as u64 * size_of::<QuantizedModelMatrix>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        self.dyn_model_matrix_buffer = ssbo;
        self.dyn_model_matrix_buffer_memory = ssbo_memory;
        self.dyn_model_matrix_buffer_mapped = context.device.map_memory(ssbo_memory, 0, size, vk::MemoryMapFlags::empty())?;
        Ok(())
    }

    unsafe fn create_static_model_matrix_buffer(&mut self, context: DeviceContext, size: vk::DeviceSize) -> Result<()> {
        if !self.static_model_matrix_buffer.is_null() {
            context.device.destroy_buffer(self.static_model_matrix_buffer, None);
        }
        
        if !self.static_model_matrix_buffer_memory.is_null() {
            context.device.free_memory(self.static_model_matrix_buffer_memory, None);
        }
        
        self.static_model_matrices_buffer_contents.resize(size as usize, QuantizedModelMatrix::default());

        let (ssbo, ssbo_memory) = ModelEngine::create_buffer(
            context,
            size as u64 * size_of::<QuantizedModelMatrix>() as u64,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        self.static_model_matrix_buffer = ssbo;
        self.static_model_matrix_buffer_memory = ssbo_memory;
        Ok(())
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

    pub unsafe fn create_model_matrix_buffers(&mut self, context: DeviceContext) -> Result<()> {
        self.0.create_dyn_model_matrix_buffer(context.clone(), MODEL_MATRIX_ALLOCATE_THRESHOLD as u64)?;
        self.0.create_static_model_matrix_buffer(context, MODEL_MATRIX_ALLOCATE_THRESHOLD as u64)?;
        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UniformBufferObject {
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuantizedModelMatrix {
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation: [u16; 4],
}