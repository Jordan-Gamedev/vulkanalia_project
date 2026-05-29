#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]


use anyhow::{anyhow, Result};
use glam::{Mat4, vec3};
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::Arc;
use std::{f32::consts::PI, time::Instant};
use std::ptr::copy_nonoverlapping as memcpy;
use std::collections::HashMap;
use bytemuck::{Pod, Zeroable};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::KhrSwapchainExtensionDeviceCommands;

use crate::components::render::Render;
use crate::components::transform::Transform;
use crate::engine::{App, ModelEngine, PresentEngine, RenderPipelineEngine, UniformBufferObject};
use super::device_context::DeviceContext;

/// The maximum number of frames that can be processed concurrently
const MAX_FRAMES_IN_FLIGHT: usize = 2;
const MAX_INDIRECT_DRAWS: usize = 1024;
const MAX_INSTANCES: usize = 16384;

const DEG_TO_RAD: f32 = PI / 180.0;

#[derive(Clone, Default)]
pub struct CommandEngine {
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub max_frames_in_flight: usize,
    pub current_frame: usize,
    pub indirect_draw_buffer: vk::Buffer,
    pub indirect_draw_buffer_memory: vk::DeviceMemory,
    pub indirect_draw_buffer_mapped: *mut c_void,
    pub indirect_draw_capacity: usize,
    pub instance_buffer: vk::Buffer,
    pub instance_buffer_memory: vk::DeviceMemory,
    pub instance_buffer_mapped: *mut c_void,
    pub instance_capacity: usize,
    start: Option<Instant>,
}

impl CommandEngine {
    pub fn destroy(&mut self, device: Device) {
        unsafe {
            self.in_flight_fences.iter().for_each(|f| device.destroy_fence(*f, None));
            self.render_finished_semaphores.iter().for_each(|f| device.destroy_semaphore(*f, None));
            self.image_available_semaphores.iter().for_each(|f| device.destroy_semaphore(*f, None));
            if self.command_pool != vk::CommandPool::null() && !self.command_buffers.is_empty() {
                device.free_command_buffers(self.command_pool, &self.command_buffers);
            }
            if self.indirect_draw_buffer_mapped != std::ptr::null_mut() && self.indirect_draw_buffer_memory != vk::DeviceMemory::null() {
                device.unmap_memory(self.indirect_draw_buffer_memory);
            }
            if self.instance_buffer_mapped != std::ptr::null_mut() && self.instance_buffer_memory != vk::DeviceMemory::null() {
                device.unmap_memory(self.instance_buffer_memory);
            }
            if self.indirect_draw_buffer != vk::Buffer::null() {
                device.destroy_buffer(self.indirect_draw_buffer, None);
            }
            if self.indirect_draw_buffer_memory != vk::DeviceMemory::null() {
                device.free_memory(self.indirect_draw_buffer_memory, None);
            }
            if self.instance_buffer != vk::Buffer::null() {
                device.destroy_buffer(self.instance_buffer, None);
            }
            if self.instance_buffer_memory != vk::DeviceMemory::null() {
                device.free_memory(self.instance_buffer_memory, None);
            }
            if self.command_pool != vk::CommandPool::null() {
                device.destroy_command_pool(self.command_pool, None);
            }
        }
    }

    pub fn begin_single_time_commands(&self, device: Device) -> Result<vk::CommandBuffer> {
        unsafe {
            // Allocate

            let info = vk::CommandBufferAllocateInfo::builder()
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_pool(self.command_pool)
                .command_buffer_count(1);
        
            let command_buffer = device.allocate_command_buffers(&info)?[0];
        
            // Begin
        
            let info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        
            device.begin_command_buffer(command_buffer, &info)?;
        
            Ok(command_buffer)
        }
    }
    
    pub fn end_single_time_commands(&self, context: DeviceContext, command_buffer: vk::CommandBuffer) -> Result<()> {
        unsafe {
            // End

            context.device.end_command_buffer(command_buffer)?;
        
            // Submit
        
            let command_buffers = &[command_buffer];
            let info = vk::SubmitInfo::builder().command_buffers(command_buffers);
        
            context.device.queue_submit(context.graphics_queue, &[info], vk::Fence::null())?;
            context.device.queue_wait_idle(context.graphics_queue)?;
        
            // Cleanup
        
            context.device.free_command_buffers(self.command_pool, &[command_buffer]);
        
            Ok(())
        }
    }

    /// Renders a frame for the Vulkan app
    pub fn render(app: &mut App) -> Result<()> {
        // Rotate non-static transforms
        let time = app.command_engine.start.unwrap().elapsed().as_secs_f32();
        let transforms: Vec<Transform> = app.world.query::<Transform>().iter().map(|r| r.0.clone()).collect();
        for transform in transforms {
            if !transform.is_static() {
                let value = transform.get_quantized_model_matrix(&app.world).unwrap();
                let position = vec3(value.position[0], value.position[1], value.position[2]);
                let rotation = glam::Quat::from_axis_angle(vec3(0.0, 1.0, 0.0), 90.0 * DEG_TO_RAD * time);
                let scale = vec3(value.scale[0], value.scale[1], value.scale[2]);
                transform.set_model_matrix(&mut app.world, position, rotation.normalize(), scale);                
            }
        }

        let (indirect_draws, instances) = CommandEngine::get_indirect_data(app);

        unsafe {
            let context = app.device_context.as_ref().clone().unwrap();
            let device = context.device;

            let size = app.present_engine.as_ref().clone().window.unwrap().inner_size();
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            let in_flight_fence = app.command_engine.in_flight_fences[app.command_engine.current_frame];

            device.wait_for_fences(&[in_flight_fence], true, u64::MAX)?;
            
            let result = device.acquire_next_image_khr(
                app.present_engine.swapchain,
                u64::MAX,
                app.command_engine.image_available_semaphores[app.command_engine.current_frame],
                vk::Fence::null(),
            );

            let image_index = match result {
                Ok((image_index, _)) => image_index as usize,
                Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                    PresentEngine::recreate_swapchain(app)?;
                    return Ok(());
                }
                Err(e) => return Err(anyhow!(e)),
            };

            let image_in_flight = app.command_engine.images_in_flight[image_index];
            if !image_in_flight.is_null() {
                device.wait_for_fences(&[image_in_flight], true, u64::MAX)?;
            }

            Arc::make_mut(&mut app.command_engine).images_in_flight[image_index] = in_flight_fence;

            app.command_engine.update_uniform_buffer(
                device.clone(),
                app.present_engine.as_ref().clone(),
                app.model_engine.as_ref().clone(),
                app.command_engine.current_frame,
            )?;

            let (command_buffer, current_frame, indirect_draw_buffer, instance_buffer, max_frames_in_flight) = {
                let command_engine = Arc::make_mut(&mut app.command_engine);
                if indirect_draws.len() > command_engine.indirect_draw_capacity {
                    return Err(anyhow!("too many indirect draws for the allocated buffer"));
                }

                if instances.len() > command_engine.instance_capacity {
                    return Err(anyhow!("too many instances for the allocated instance buffer"));
                }
                if !indirect_draws.is_empty() {
                    memcpy(
                        indirect_draws.as_ptr(),
                        command_engine.indirect_draw_buffer_mapped.cast::<IndirectDrawData>(),
                        indirect_draws.len(),
                    );
                }

                if !instances.is_empty() {
                    memcpy(
                        instances.as_ptr(),
                        command_engine.instance_buffer_mapped.cast::<PerInstanceData>(),
                        instances.len(),
                    );
                }

                (
                    command_engine.command_buffers[image_index],
                    command_engine.current_frame,
                    command_engine.indirect_draw_buffer,
                    command_engine.instance_buffer,
                    command_engine.max_frames_in_flight,
                )
            };

            // Commands

            device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

            let info = vk::CommandBufferBeginInfo::builder();
            device.begin_command_buffer(command_buffer, &info)?;

            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(app.present_engine.swapchain_extent);

            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };

            let depth_clear_value = vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            };

            let clear_values = &[color_clear_value, depth_clear_value];
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(app.rp_engine.render_pass)
                .framebuffer(app.rp_engine.framebuffers[image_index])
                .render_area(render_area)
                .clear_values(clear_values);

            device.cmd_begin_render_pass(command_buffer, &info, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, app.rp_engine.pipeline);

            if !indirect_draws.is_empty() {
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[app.model_engine.vertex_buffer], &[0]);
                device.cmd_bind_index_buffer(command_buffer, app.model_engine.index_buffer, 0, vk::IndexType::UINT32);
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    app.rp_engine.pipeline_layout,
                    0,
                    &[app.rp_engine.descriptor_sets[current_frame]],
                    &[],
                );
                device.cmd_draw_indexed_indirect(
                    command_buffer,
                    indirect_draw_buffer,
                    0,
                    indirect_draws.len() as u32,
                    size_of::<IndirectDrawData>() as u32,
                );
            }

            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;

            let wait_semaphores = &[app.command_engine.image_available_semaphores[current_frame]];
            let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = &[command_buffer];
            let signal_semaphores = &[app.command_engine.render_finished_semaphores[image_index]];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(wait_stages)
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            device.reset_fences(&[in_flight_fence])?;

            device.queue_submit(context.graphics_queue, &[submit_info], in_flight_fence)?;

            let swapchains = &[app.present_engine.swapchain];
            let image_indices = &[image_index as u32];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            let result = device.queue_present_khr(context.present_queue, &present_info);

            let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
                || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

            if app.present_engine.resized || changed {
                Arc::make_mut(&mut app.present_engine).resized = false;
                PresentEngine::recreate_swapchain(app)?;
            } else if let Err(e) = result {
                return Err(anyhow!(e));
            }

            Arc::make_mut(&mut app.command_engine).current_frame = (current_frame + 1) % max_frames_in_flight;

            Ok(())
        }
    }

    /// Gets the IndirectDrawData and PerInstanceData from the render and transform components
    fn get_indirect_data(app: &mut App) -> (Vec<IndirectDrawData>, Vec<PerInstanceData>) {
        // Group instances by model name, collect per-instance data
        let mut instances_map: HashMap<String, Vec<PerInstanceData>> = HashMap::new();

        for (render, transform, entity) in app.world.query2::<Render, Transform>() {
            let model = if let Some(model) = app.model_engine.loaded_models.get(&render.model_name) {
                *model
            } else {
                continue;
            };

            let texture_slot_index = app
                .texture_engine
                .get_texture_slot_index(&render.material.albedo_name)
                .unwrap_or(0);

            let entry = PerInstanceData {
                model_matrix_info: transform.model_matrix_info,
                texture_index: texture_slot_index,
                padding0: 0,
                padding1: 0,
            };

            instances_map.entry(render.model_name.clone()).or_default().push(entry);
        }

        let mut indirect_draws = Vec::<IndirectDrawData>::new();
        let mut instances = Vec::<PerInstanceData>::new();
        let mut base_instance_index: u32 = 0;

        for (model_name, model_instance_list) in instances_map.iter() {
            if let Some(model) = app.model_engine.loaded_models.get(model_name) {
                let count = model_instance_list.len() as u32;
                indirect_draws.push(IndirectDrawData {
                    index_count: model.index_length,
                    instance_count: count,
                    first_index: model.index_offset,
                    vertex_offset: model.vertex_offset as i32,
                    first_instance: base_instance_index,
                    model_matrix_info: 0,
                    texture_index: 0,
                    padding: 0,
                });

                instances.extend(model_instance_list.iter());
                base_instance_index += count;
            }
        }

        (indirect_draws, instances)
    }

    /// Updates the uniform buffer object for the Vulkan app
    unsafe fn update_uniform_buffer(&self, device: Device, present_engine: PresentEngine, model_engine: ModelEngine, frame_index: usize) -> Result<()> {
        // MVP

        let time = self.start.unwrap().elapsed().as_secs_f32();

        let view = Mat4::look_at_rh(
            vec3(2.0, 2.0, 2.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0)
        );

        let mut proj = glam::Mat4::perspective_rh(
            45.0 * DEG_TO_RAD,
            present_engine.swapchain_extent.width as f32 / present_engine.swapchain_extent.height as f32,
            0.1,
            10.0,
        );

        proj.col_mut(1).y *= -1.0;

        let ubo = UniformBufferObject { view, proj };

        // Copy

        let memory = device.map_memory(
            model_engine.uniform_buffers_memory[frame_index],
            0,
            size_of::<UniformBufferObject>() as u64,
            vk::MemoryMapFlags::empty(),
        )?;

        memcpy(&ubo, memory.cast(), 1);

        device.unmap_memory(model_engine.uniform_buffers_memory[frame_index]);

        Ok(())
    }
}

pub struct CommandEngineBuilder(pub(crate) CommandEngine);

impl CommandEngineBuilder {
    pub fn new() -> Self {
        let mut command_engine = CommandEngine::default();
        command_engine.max_frames_in_flight = MAX_FRAMES_IN_FLIGHT;
        Self(command_engine)
    }

    pub unsafe fn create_command_pool(&mut self, context: DeviceContext) -> Result<()> {
        let info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(context.graphics_queue_family_index);
    
        self.0.command_pool = context.device.create_command_pool(&info, None)?;
    
        Ok(())
    }

    pub unsafe fn create_indirect_draw_buffer(&mut self, context: DeviceContext) -> Result<()> {
        let size = (size_of::<IndirectDrawData>() * MAX_INDIRECT_DRAWS) as u64;
        let (buffer, memory) = ModelEngine::create_buffer(
            context.clone(),
            size,
            vk::BufferUsageFlags::INDIRECT_BUFFER | vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        self.0.indirect_draw_buffer = buffer;
        self.0.indirect_draw_buffer_memory = memory;
        self.0.indirect_draw_buffer_mapped = context.device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?;
        self.0.indirect_draw_capacity = MAX_INDIRECT_DRAWS;

        Ok(())
    }

    pub unsafe fn create_instance_buffer(&mut self, context: DeviceContext) -> Result<()> {
        let size = (std::mem::size_of::<PerInstanceData>() * MAX_INSTANCES) as u64;
        let (buffer, memory) = ModelEngine::create_buffer(
            context.clone(),
            size,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        self.0.instance_buffer = buffer;
        self.0.instance_buffer_memory = memory;
        self.0.instance_buffer_mapped = context.device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty())?;
        self.0.instance_capacity = MAX_INSTANCES;

        Ok(())
    }

    pub unsafe fn create_command_buffers(&mut self, device: Device, present_engine: PresentEngine, rp_engine: RenderPipelineEngine) -> Result<()> {
        // Allocate
    
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.0.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(rp_engine.framebuffers.len() as u32);
    
        self.0.command_buffers = device.allocate_command_buffers(&allocate_info)?;
    
        Ok(())
    }

    pub unsafe fn create_sync_objects(&mut self, device: Device, present_engine: PresentEngine) -> Result<()> {
        self.0.max_frames_in_flight = MAX_FRAMES_IN_FLIGHT;
        self.0.start = Some(Instant::now());

        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
    
        for _ in 0..self.0.max_frames_in_flight {
            self.0.image_available_semaphores
                .push(device.create_semaphore(&semaphore_info, None)?);
    
            self.0.in_flight_fences.push(device.create_fence(&fence_info, None)?);
        }
    
        for _ in 0..present_engine.swapchain_images.len() {
            self.0.render_finished_semaphores
                .push(device.create_semaphore(&semaphore_info, None)?);
        }
    
        self.0.images_in_flight = present_engine.swapchain_images.iter().map(|_| vk::Fence::null()).collect();
    
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct IndirectDrawData {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
    pub model_matrix_info: u32,
    pub texture_index: u32,
    pub padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PerInstanceData {
    pub model_matrix_info: u32,
    pub texture_index: u32,
    pub padding0: u32,
    pub padding1: u32,
}