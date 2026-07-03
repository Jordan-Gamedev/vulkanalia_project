#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::{anyhow, Result};
use bytemuck::Zeroable;
use vulkanalia::prelude::v1_0::*;
use core::slice;
use std::collections::HashMap;
use std::mem::size_of;
use glam::{Mat4, Quat, Vec2, Vec3, vec2, vec3};

use crate::engine::buffers::Buffer;
use crate::engine::command_engine::{IndirectDrawData, PerInstanceData};
use crate::engine::texture_engine::{SamplerContents};
use crate::engine::{App, CommandEngine, RenderPipelineEngine};
use crate::resources::{AssetId, get_asset_from_id};

use super::device_context::DeviceContext;

// Allocates/Deallocates 32KiB at a time
const MODEL_MATRIX_ALLOCATE_THRESHOLD: u32 = 1024;

#[derive(Clone, Default)]
pub struct ModelEngine {
    // Model Shape
    pub vertex_buffer: Buffer<QuantizedVertex>,
    pub index_buffer: Buffer<u32>,
    
    // Camera
    pub uniform_buffers: Vec<Buffer<UniformBufferObject>>,

    // Transforms
    pub dyn_model_matrix_buffer: Buffer<QuantizedModelMatrix>,

    // Static Transforms
    pub static_model_matrix_buffer: Buffer<QuantizedModelMatrix>,

    // Actively loaded models
    pub loaded_models: HashMap<(AssetId, AssetId), Model>,
}

impl ModelEngine {
    pub fn destroy(&mut self, device: Device) {
        // Destroy vertex and index
        self.vertex_buffer.destroy(&device);
        self.index_buffer.destroy(&device);
        // Destroy ubo
        self.uniform_buffers.iter_mut().for_each(|b| b.destroy(&device));
        // Destroy ssbos
        self.dyn_model_matrix_buffer.destroy(&device);
        self.static_model_matrix_buffer.destroy(&device);
    }

    pub fn load_model(&mut self, context: DeviceContext, command_engine: CommandEngine, vertex_asset_id: AssetId, index_asset_id: AssetId) -> Result<()> {
        if vertex_asset_id == AssetId::None || index_asset_id == AssetId::None {
            return Ok(())
        }

        // Do not load model if it is already loaded
        if self.loaded_models.contains_key(&(vertex_asset_id, index_asset_id)) {
            return Ok(())
        }
        
        // Get vertices
        
        let vertex_asset = get_asset_from_id(vertex_asset_id);
        let vertex_bytes: &[u8] = &vertex_asset.0[size_of::<u32>()..];
        let vertex_count = u32::from_be_bytes(vertex_asset.0[0..size_of::<u32>()].try_into().unwrap()) as usize;

        let vertices: Vec<QuantizedVertex> = match meshopt::decode_vertex_buffer(vertex_bytes, vertex_count) {
            Ok(bytes) => bytes,
            Err(_) => return Err(anyhow!("Failed to decode vertex buffer")),
        };
    
        // Get indices
        
        let index_asset = get_asset_from_id(index_asset_id);
        let index_bytes: &[u8] = &index_asset.0[size_of::<u32>()..];
        let index_count = u32::from_be_bytes(index_asset.0[0..size_of::<u32>()].try_into().unwrap()) as usize;
        let indices: Vec<u32> = match meshopt::decode_index_buffer(index_bytes, index_count) {
            Ok(indices) => indices,
            Err(_) => return Err(anyhow!("Failed to decode index buffer")),
        };

        unsafe {
            let prev_vertex_count = self.vertex_buffer.element_count;
            let prev_index_count = self.index_buffer.element_count;
            self.vertex_buffer.add_items(context.clone(), &command_engine, vertices)?;
            self.index_buffer.add_items(context, &command_engine, indices)?;
            let model = Model {
                vertex_offset: prev_vertex_count,
                vertex_length: vertex_count as u32,
                index_offset: prev_index_count,
                index_length: index_count as u32,
                indirect_draw_data_ptr: command_engine.indirect_draw_buffer_mapped.add(self.loaded_models.len()),
            };

            let mut last_draw_data: *mut IndirectDrawData = std::ptr::null_mut();
            for i in (0..command_engine.indirect_draw_capacity).rev() {
                let draw_data = command_engine.indirect_draw_buffer_mapped.add(i);
                if draw_data.read().instance_count > 0 {
                    last_draw_data = draw_data;
                    break;
                }
            }

            let mut first_instance: u32 = 0;
            if !last_draw_data.is_null() {
                first_instance = last_draw_data.read().first_instance + last_draw_data.read().instance_count;
            }

            *model.indirect_draw_data_ptr = IndirectDrawData {
                index_count: index_count as u32,
                instance_count: 0,
                first_index: prev_index_count,
                vertex_offset: prev_vertex_count as i32,
                first_instance: first_instance,
            };
            self.loaded_models.insert((vertex_asset_id, index_asset_id), model);
        }

        Ok(())
    }

    pub fn unload_model(&mut self, context: DeviceContext, command_engine: CommandEngine, vertex_asset_id: AssetId, index_asset_id: AssetId) -> Result<()> {
        if vertex_asset_id == AssetId::None || index_asset_id == AssetId::None {
            return Ok(())
        }

        let unloading_model = self.loaded_models.get_mut(&(vertex_asset_id, index_asset_id)).expect("Model not found").clone();        
        let fully_unloaded = unsafe { unloading_model.indirect_draw_data_ptr.read().instance_count <= 1 };

        if fully_unloaded {
            self.vertex_buffer.remove_items(context.clone(), &command_engine, unloading_model.vertex_offset, unloading_model.vertex_offset + unloading_model.vertex_length)?;
            self.index_buffer.remove_items(context.clone(), &command_engine, unloading_model.index_offset, unloading_model.index_offset + unloading_model.index_length)?;

            // Update other model offsets
            self.loaded_models.values_mut()
                .filter(|m| m.vertex_offset > unloading_model.vertex_offset)
                .for_each(|m| {
                    m.vertex_offset -= unloading_model.vertex_length;
                    m.index_offset -= unloading_model.index_length;
                    let draw_data = unsafe { m.indirect_draw_data_ptr.as_mut().unwrap() };
                    draw_data.vertex_offset = m.vertex_offset as i32;
                    draw_data.first_index = m.index_offset;
                });
        }
        
        Ok(())
    }

    pub fn create_instance(app: &mut App, vertex_asset_id: AssetId, index_asset_id: AssetId, texture_asset_id: AssetId, sampler_contents: SamplerContents, model_matrix_info: u32) -> Result<*mut PerInstanceData> {
        // Potentially load model and texture
        app.load_model(vertex_asset_id, index_asset_id)?;
        app.load_texture(texture_asset_id, sampler_contents)?;

        // Get model that the instance uses
        let model = app.model_engine.loaded_models.get(&(vertex_asset_id, index_asset_id)).unwrap().clone();
        
        // Get bindless texture index
        let tex_index = app.texture_engine
            .get_texture_slot_index(texture_asset_id)
            .unwrap_or(0);

        // Get bindless sampler index
        let sampler_index = app.texture_engine
            .get_sampler_slot_index(sampler_contents)
            .unwrap_or(0);

        unsafe {
            let mut affected_draw_data: Vec<*mut IndirectDrawData> = app.model_engine.loaded_models
                .iter()
                .filter(|&(_, m)| m.indirect_draw_data_ptr.read().first_instance > model.indirect_draw_data_ptr.read().first_instance)
                .map(|(_, m)| m.indirect_draw_data_ptr)
                .collect();
            affected_draw_data.sort_unstable_by(|a, b| a.read().first_instance.cmp(&b.read().first_instance));

            // Add one to the instance offset for those found in the buffer after the new instance
            // and continuously swap beginnings of instance buffer sections to make room for new instance
            if affected_draw_data.len() > 0 {
                let mut saved_instance: PerInstanceData = app.command_engine.instance_buffer_mapped.add(affected_draw_data[0].read().first_instance as usize).read();
                for draw_data in affected_draw_data {
                    let draw_data = draw_data.as_mut().unwrap();
                    let next_instance_ptr = app.command_engine.instance_buffer_mapped.add((draw_data.first_instance + draw_data.instance_count) as usize);
    
                    let next_instance = next_instance_ptr.read();
                    *next_instance_ptr = saved_instance;
                    saved_instance = next_instance;
    
                    draw_data.first_instance += 1;
                }
            }
            
            // Add instance to instance buffer
            let new_instance = PerInstanceData {
                model_matrix_info: model_matrix_info,
                texture_index: tex_index,
                sampler_index: sampler_index,
                padding: if vertex_asset_id == AssetId::CubeVertices { 0 } else if vertex_asset_id == AssetId::LimpetVertices { 1 } else { 2 },
            };
            let draw_data = model.indirect_draw_data_ptr.as_mut().unwrap();
            let new_instance_ptr = app.command_engine.instance_buffer_mapped.add((draw_data.first_instance + draw_data.instance_count) as usize);
            *new_instance_ptr = new_instance;
            draw_data.instance_count += 1;

            // for i in 0..app.command_engine.indirect_draw_capacity {
            //     println!("after instance created {:?}: {:?}", vertex_asset_id, app.command_engine.indirect_draw_buffer_mapped.add(i).read());
            // }

            // for i in 0..app.command_engine.instance_capacity {
            //     if i % 4 == 0 {
            //         println!();
            //     }
            //     println!("{:?}", app.command_engine.instance_buffer_mapped.add(i).read());
            // }
            // println!();

            Ok(new_instance_ptr)
        }
    }

    pub fn remove_instance(app: &mut App, vertex_asset_id: AssetId, index_asset_id: AssetId, texture_asset_id: AssetId, sampler_contents: SamplerContents, instance: *mut PerInstanceData) -> Result<()> {
        let temp_instance_model_info = unsafe { instance.read().model_matrix_info };
        
        // Potentially unload model and texture
        app.unload_model(vertex_asset_id, index_asset_id)?;
        app.unload_texture(texture_asset_id, sampler_contents)?;
        
        // Get model that the instance uses
        let model = app.model_engine.loaded_models.get(&(vertex_asset_id, index_asset_id)).unwrap().clone();

        let draw_data = unsafe { model.indirect_draw_data_ptr.as_mut().unwrap() };
        
        unsafe {
            // Replace the removed instance with the instance at the end of the instance buffer section
            draw_data.instance_count -= 1;
            *instance = app.command_engine.instance_buffer_mapped.add((draw_data.first_instance + draw_data.instance_count) as usize).read();
        
            // Get draw datas affected by this removal
            let mut affected_draw_data: Vec<*mut IndirectDrawData> = app.model_engine.loaded_models
                .iter()
                .filter(|&(_, m)| m.indirect_draw_data_ptr.read().first_instance > draw_data.first_instance)
                .map(|(_, m)| m.indirect_draw_data_ptr)
                .collect();
            affected_draw_data.sort_unstable_by(|a, b| a.read().first_instance.cmp(&b.read().first_instance));

            // Remove one from the instance offset for those found in the buffer after the new instance
            // and continuously overwrite ends of instance buffer sections with the next ends to cover the empty instance
            let mut instance_to_overwrite: *mut PerInstanceData = app.command_engine.instance_buffer_mapped.add((draw_data.first_instance + draw_data.instance_count) as usize);
            for draw_data in affected_draw_data {
                let draw_data = draw_data.as_mut().unwrap();
                draw_data.first_instance -= 1;

                let this_instance = app.command_engine.instance_buffer_mapped.add((draw_data.first_instance + draw_data.instance_count) as usize);
                *instance_to_overwrite = this_instance.read();
                *this_instance = PerInstanceData {
                    model_matrix_info: u32::MAX,
                    texture_index: u32::MAX,
                    sampler_index: u32::MAX,
                    padding: u32::MAX,
                };
                instance_to_overwrite = this_instance;
            }

            // Unload model if there are no more instances using it
            if draw_data.instance_count == 0 {
                let mut last_draw_data: *mut IndirectDrawData = std::ptr::null_mut();
                for i in (0..app.command_engine.indirect_draw_capacity).rev() {
                    let draw_data = app.command_engine.indirect_draw_buffer_mapped.add(i);
                    if draw_data.read().instance_count > 0 {
                        last_draw_data = draw_data;
                        break;
                    }
                }

                let model_engine = std::sync::Arc::make_mut(&mut app.model_engine);
                model_engine.loaded_models
                    .iter_mut()
                    .find(|(_, m)| m.indirect_draw_data_ptr == last_draw_data)
                    .map(|(_, m)| m.indirect_draw_data_ptr = model.indirect_draw_data_ptr);

                *model.indirect_draw_data_ptr = last_draw_data.read();
                *last_draw_data = IndirectDrawData::zeroed();

                model_engine.loaded_models.remove(&(vertex_asset_id, index_asset_id));
            }
        
            // for i in 0..app.command_engine.indirect_draw_capacity {
            //     println!("after instance {temp_instance_model_info} removed {:?}: {:?}", vertex_asset_id, app.command_engine.indirect_draw_buffer_mapped.add(i).read());
            // }

            // for i in 0..app.command_engine.instance_capacity {
            //     if i % 4 == 0 {
            //         println!();
            //     }
            //     println!("{:?}", app.command_engine.instance_buffer_mapped.add(i).read());
            // }
            // println!();
        }

        Ok(())
    }

    pub fn create_model_matrix(app: &mut App, matrix: QuantizedModelMatrix, is_static: bool) -> Result<u32> {
        let model_engine = std::sync::Arc::make_mut(&mut app.model_engine);
        
        if is_static {
            let prev_buffer = model_engine.static_model_matrix_buffer.buffer;
            let chosen_index = model_engine.static_model_matrix_buffer.add_item(app.device_context.as_ref().clone().unwrap(), &app.command_engine, matrix)?;
            let new_buffer = model_engine.static_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                model_engine.update_model_matrix_buffer_descriptors(app.device_context.as_ref().clone().unwrap().device, app.rp_engine.as_ref().clone(), app.command_engine.as_ref().clone())?;
            }
            return Ok(chosen_index)
        } else {
            let prev_buffer = model_engine.dyn_model_matrix_buffer.buffer;
            let chosen_index = model_engine.dyn_model_matrix_buffer.add_item(app.device_context.as_ref().clone().unwrap(), &app.command_engine, matrix)?;
            let new_buffer = model_engine.dyn_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                model_engine.update_model_matrix_buffer_descriptors(app.device_context.as_ref().clone().unwrap().device, app.rp_engine.as_ref().clone(), app.command_engine.as_ref().clone())?;
            }
            Ok(chosen_index)
        }
    }

    pub fn remove_model_matrix(app: &mut App, model_matrix_index: u32, is_static: bool) -> Result<()> {
        let model_engine = std::sync::Arc::make_mut(&mut app.model_engine);
        
        if is_static { 
            let prev_buffer = model_engine.static_model_matrix_buffer.buffer;
            model_engine.static_model_matrix_buffer.remove_item_at(app.device_context.as_ref().clone().unwrap(), &app.command_engine, model_matrix_index)?;
            let new_buffer = model_engine.static_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                model_engine.update_model_matrix_buffer_descriptors(app.device_context.as_ref().clone().unwrap().device, app.rp_engine.as_ref().clone(), app.command_engine.as_ref().clone())?;
            }
        } else {
            let prev_buffer = model_engine.dyn_model_matrix_buffer.buffer;
            model_engine.dyn_model_matrix_buffer.remove_item_at(app.device_context.as_ref().clone().unwrap(), &app.command_engine, model_matrix_index)?;
            let new_buffer = model_engine.dyn_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                model_engine.update_model_matrix_buffer_descriptors(app.device_context.as_ref().clone().unwrap().device, app.rp_engine.as_ref().clone(), app.command_engine.as_ref().clone())?;
            }
        }
        
        Ok(())
    }

    pub fn get_model_matrix(&self, model_matrix_index: u32, is_static: bool) -> QuantizedModelMatrix {
        if is_static {
            unsafe { self.static_model_matrix_buffer.mapped.add(model_matrix_index as usize).read() }
        } else {
            unsafe { self.dyn_model_matrix_buffer.mapped.add(model_matrix_index as usize).read() }
        }
    }

    pub fn get_model_matrix_mut(&self, model_matrix_index: u32, is_static: bool) -> &mut QuantizedModelMatrix {
        if is_static {
            unsafe { self.static_model_matrix_buffer.mapped.add(model_matrix_index as usize).as_mut().unwrap() }
        } else {
            unsafe { self.dyn_model_matrix_buffer.mapped.add(model_matrix_index as usize).as_mut().unwrap() }
        }
    }

    pub fn set_model_matrix(&mut self, model_matrix_index: u32, position: Vec3, rotation: Quat, scale: Vec3, is_static: bool) -> Result<()> {
        let buffer_ptr = if is_static { self.static_model_matrix_buffer.mapped } else { self.dyn_model_matrix_buffer.mapped };        
        let model_matrix = unsafe { buffer_ptr.add(model_matrix_index as usize).as_mut().unwrap() };

        model_matrix.position = position.to_array();
        model_matrix.scale = scale.to_array();
        let rotation_i16: [i16; 4] = [
            (rotation.x * i16::MAX as f32) as i16,
            (rotation.y * i16::MAX as f32) as i16,
            (rotation.z * i16::MAX as f32) as i16,
            (rotation.w * i16::MAX as f32) as i16,
        ];
        model_matrix.rotation = rotation_i16;

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
    
    pub fn get_buffer_contents<T: std::fmt::Debug + Clone>(context: DeviceContext, command_engine: &CommandEngine, buffer: vk::Buffer, content_length: usize) -> Result<Vec<T>> {
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
            let vec: Vec<T> = slice::from_raw_parts(memory.cast(), content_length).to_vec();

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

    pub fn update_model_matrix_buffer_descriptors(&mut self, device: Device, rp_engine: RenderPipelineEngine, command_engine: CommandEngine) -> Result<()> {
        if rp_engine.descriptor_sets.is_empty() {
            return Ok(());
        }

        //unsafe { device.device_wait_idle()?; }

        for i in 0..command_engine.max_frames_in_flight {
            let static_model_matrix_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.static_model_matrix_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let static_model_matrix_buffer_info = [static_model_matrix_info];
            let static_model_matrix_write = vk::WriteDescriptorSet::builder()
                .dst_set(rp_engine.descriptor_sets[i])
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&static_model_matrix_buffer_info);

            let dyn_model_matrix_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.dyn_model_matrix_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let dyn_model_matrix_buffer_info = [dyn_model_matrix_info];
            let dyn_model_matrix_write = vk::WriteDescriptorSet::builder()
                .dst_set(rp_engine.descriptor_sets[i])
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&dyn_model_matrix_buffer_info);

            unsafe {
                device.update_descriptor_sets(
                    &[static_model_matrix_write, dyn_model_matrix_write],
                    &[] as &[vk::CopyDescriptorSet]
                );
            }
        }

        Ok(())
    }
}

pub struct ModelEngineBuilder(pub(crate) ModelEngine);

impl ModelEngineBuilder {
    pub fn new() -> Self {
        Self(ModelEngine::default())
    }

    pub fn create_vertex_index_buffers(&mut self, context: DeviceContext, command_engine: &CommandEngine) {
        self.0.vertex_buffer = Buffer::new(context.clone(), command_engine, 1, vk::BufferUsageFlags::VERTEX_BUFFER, vk::MemoryPropertyFlags::DEVICE_LOCAL, 0, Vec::new());
        self.0.index_buffer = Buffer::new(context, command_engine, 1, vk::BufferUsageFlags::INDEX_BUFFER, vk::MemoryPropertyFlags::DEVICE_LOCAL, 0, Vec::new());
    }

    pub unsafe fn create_uniform_buffers(&mut self, context: DeviceContext, command_engine: CommandEngine) -> Result<()> {
        let device = context.clone().device;
        self.0.uniform_buffers.iter_mut().for_each(|b| b.destroy(&device));
        self.0.uniform_buffers.clear();

        for _ in 0..command_engine.max_frames_in_flight {
            let buffer: Buffer<UniformBufferObject> = Buffer::new(context.clone(), &command_engine, 1, vk::BufferUsageFlags::UNIFORM_BUFFER, vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE, 0, Vec::new());
            self.0.uniform_buffers.push(buffer);
        }
    
        Ok(())
    }

    pub unsafe fn create_model_matrix_buffers(&mut self, context: DeviceContext, command_engine: &CommandEngine) -> Result<()> {
        self.0.dyn_model_matrix_buffer = Buffer::new(context.clone(), command_engine, MODEL_MATRIX_ALLOCATE_THRESHOLD as u64, vk::BufferUsageFlags::STORAGE_BUFFER, vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE, MODEL_MATRIX_ALLOCATE_THRESHOLD, Vec::new());
        self.0.static_model_matrix_buffer = Buffer::new(context.clone(), command_engine, MODEL_MATRIX_ALLOCATE_THRESHOLD as u64, vk::BufferUsageFlags::STORAGE_BUFFER, vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE, MODEL_MATRIX_ALLOCATE_THRESHOLD, Vec::new());
        Ok(())
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
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
    pub indirect_draw_data_ptr: *mut IndirectDrawData,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuantizedModelMatrix {
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation: [i16; 4],
}