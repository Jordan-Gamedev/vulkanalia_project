#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use crate::engine::CommandHandle;
use crate::engine::DescriptorHandle;
use crate::engine::DeviceContext;
use crate::engine::DeviceQueueHandle;
use crate::engine::IndirectDrawData;
use crate::engine::Mesh;
use crate::engine::MeshBufferLayout;
use crate::engine::MeshMetadata;
use crate::engine::ModelHandle;
use crate::engine::PerInstanceData;
use crate::engine::PresentHandle;
use crate::engine::PushConstant;
use crate::engine::QuantizedModelMatrix;
use crate::engine::QuantizedVertex;
use crate::engine::RenderPipelineHandle;
use crate::engine::SamplerContents;
use crate::engine::SamplerUsage;
use crate::engine::SwapchainHandle;
use crate::engine::SyncHandle;
use crate::engine::Texture;
use crate::engine::TextureHandle;
use crate::engine::TextureUsage;
use crate::engine::UniformBufferObject;
use crate::engine::Visbuffer;
use crate::engine::WindowHandle;
use crate::engine::buffers::Buffer;
use crate::resources::AssetId;
use crate::resources::get_asset_from_id;
use anyhow::{Result, anyhow};
use glam::Quat;
use glam::Vec3;
use glam::{Mat4, vec3};
use log::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::f32::consts::PI;
use std::ffi::CStr;
use std::fmt::Debug;
use std::os::raw::c_void;
use std::ptr::copy_nonoverlapping as memcpy;
use std::sync::Arc;
use thiserror::Error;
use vulkanalia::Version;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LIBRARY, LibloadingLoader};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ExtDebugUtilsExtensionInstanceCommands;
use vulkanalia::vk::{KhrSurfaceExtensionInstanceCommands, KhrSwapchainExtensionDeviceCommands};
use vulkanalia::window as vk_window;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Fullscreen, Window, WindowBuilder};

/// The maximum number of frames that can be processed concurrently
const MAX_FRAMES_IN_FLIGHT: u32 = 3;

#[derive(Debug, Error)]
#[error("{0}")]
struct SuitabilityError(&'static str);

#[derive(bevy_ecs::resource::Resource)]
#[derive(Clone)]
pub struct VulkanRenderer {
    pub device_context: DeviceContext,
    pub present_handle: PresentHandle,
    pub render_pipeline_handle: RenderPipelineHandle,
    pub command_handle: CommandHandle,
    pub model_handle: ModelHandle,
    pub texture_handle: TextureHandle,
}

impl VulkanRenderer {
    pub fn new() -> Result<Self> {
        unsafe {
            // Create window
            let (event_loop, window) = create_window(true)?;

            // Create vulkan entry point
            let entry = create_entry()?;

            // Create vulkan instance and messenger for debugging
            let (instance, messenger) = create_instance(&window, &entry)?;

            // Create window surface
            let surface = create_surface(instance.clone(), &window)?;

            // Finalize window handle
            let window_handle = WindowHandle {
                window: Arc::new(window),
                event_loop: Some(Arc::new(event_loop)),
                surface,
                is_resized: false,
            };

            // Get physical Device
            let physical_device = pick_physical_device(instance.clone(), surface)?;

            // Create logical device
            let device =
                create_logical_device(messenger, &entry, &instance, physical_device, surface)?;

            // Get device queues
            let device_queue_handle =
                get_device_graphics_present_queues(device.clone(), &instance, physical_device, &surface)?;

            // Finalize device context
            let device_context = DeviceContext {
                messenger,
                entry,
                instance: instance.clone(),
                device: device.clone(),
                physical_device,
                device_queue_handle: device_queue_handle.clone(),
            };

            // Set a starting value for multisample antialiasing
            let msaa_samples = set_default_msaa(&device_context);

            // Create the window's swapchain
            let swapchain_handle = create_swapchain(&device_context, &window_handle.window, surface)?;

            // Create screen color texture
            let color_texture = create_color_texture(
                &device_context,
                &swapchain_handle,
                msaa_samples
            )?;

            // Create screen depth texture
            let depth_texture = create_depth_texture(
                &device_context,
                &swapchain_handle,
                msaa_samples,
            )?;

            // Finalize present handle
            let present_handle = PresentHandle {
                window_handle,
                swapchain_handle: swapchain_handle.clone(),
                color_texture,
                depth_texture,
                msaa_samples,
            };

            // Create base render pass
            let base_render_pass = create_base_render_pass(
                &device_context,
                &swapchain_handle,
                msaa_samples,
            )?;

            // Create a descriptor set layout for gpu objects
            let descriptor_set_layout = create_descriptor_set_layout(device.clone())?;

            // Create a descriptor pool
            let descriptor_pool = create_descriptor_pool(device.clone())?;

            // Create the render pipeline
            let (pipeline, pipeline_layout) = create_pipeline(
                &swapchain_handle,
                msaa_samples,
                descriptor_set_layout,
                base_render_pass,
                &device_context,
            )?;

            // Create framebuffers
            let framebuffers = create_framebuffers(
                &swapchain_handle,
                color_texture,
                depth_texture,
                base_render_pass,
                device.clone(),
            )?;

            // Create command pool
            let command_pool = create_command_pool(device.clone(), device_queue_handle)?;

            // Create visbuffers
            let main_camera_visbuffers = create_visbuffers(&device_context, command_pool);

            // Create source instance buffer that stores all active instances
            let source_instance_buffer = create_source_instance_buffer(&device_context, command_pool);

            // Create vertex and index buffers
            let (vertex_buffer, index_buffer) =
                create_vertex_index_buffers(device_context.clone(), command_pool);

            // Create uniform buffer objects
            let uniform_buffers = create_uniform_buffers(&device_context, command_pool);

            // Create both dynamic and static model matrix storage buffers
            let (dyn_model_matrix_buffer, static_model_matrix_buffer) =
                create_model_matrix_buffers(device_context.clone(), command_pool);

            // Create mesh buffer
            let mesh_uniform_buffer = create_mesh_buffer(device_context.clone(), command_pool);

            // Finalize model handle
            let model_handle = ModelHandle {
                vertex_buffer,
                index_buffer,
                uniform_buffers,
                dyn_model_matrix_buffer,
                static_model_matrix_buffer,
                loaded_meshes: HashMap::new(),
                mesh_uniform_buffer: mesh_uniform_buffer,
            };

            // Create starting texture handle
            let texture_handle = TextureHandle::default();

            // Create descriptor sets
            let descriptor_sets = create_descriptor_sets(
                &device_context,
                &model_handle,
                &main_camera_visbuffers,
                &texture_handle,
                descriptor_set_layout,
                descriptor_pool,
            )?;

            // Finalize descriptor handle
            let descriptor_handle = DescriptorHandle {
                descriptor_set_layout,
                descriptor_pool,
                descriptor_sets,
            };

            // Finalize render pipeline handle
            let render_pipeline_handle = RenderPipelineHandle {
                base_render_pass,
                descriptor_handle,
                pipeline,
                pipeline_layout,
                framebuffers: framebuffers.clone(),
            };

            // Create command buffers
            let command_buffers = create_command_buffers(device.clone(), command_pool, framebuffers.len())?;

            // Create sync objects
            let sync_handle = create_sync_objects(device, &swapchain_handle)?;

            // Finalize command handle
            let command_handle = CommandHandle {
                command_pool,
                command_buffers,
                sync_handle,
                source_instance_buffer,
                main_camera_visbuffers,
            };

            Ok(Self {
                device_context,
                present_handle,
                render_pipeline_handle,
                command_handle,
                model_handle,
                texture_handle,
            })
        }
    }

    pub fn destroy(&mut self) {
        let device = self.device_context.device.clone();
        
        unsafe {
            device.device_wait_idle().unwrap();

            // Present Handle

            self.present_handle.depth_texture.destroy(device.clone());
            self.present_handle.color_texture.destroy(device.clone());
            self.present_handle.swapchain_handle.image_views
                .iter()
                .for_each(|v| device.destroy_image_view(*v, None));
            device.destroy_swapchain_khr(self.present_handle.swapchain_handle.swapchain, None);
        
            // Render Pipeline Handle

            self.render_pipeline_handle.framebuffers
                .iter()
                .for_each(|f| device.destroy_framebuffer(*f, None));
            device.destroy_pipeline(self.render_pipeline_handle.pipeline, None);
            device.destroy_pipeline_layout(self.render_pipeline_handle.pipeline_layout, None);
            device.destroy_descriptor_pool(self.render_pipeline_handle.descriptor_handle.descriptor_pool, None);
            device.destroy_render_pass(self.render_pipeline_handle.base_render_pass, None);
            device.destroy_descriptor_set_layout(self.render_pipeline_handle.descriptor_handle.descriptor_set_layout, None);
        
            // Command Handle

            self.command_handle.sync_handle.in_flight_fences
                .iter()
                .for_each(|f| device.destroy_fence(*f, None));
            self.command_handle.sync_handle.render_finished_semaphores
                .iter()
                .for_each(|f| device.destroy_semaphore(*f, None));
            self.command_handle.sync_handle.image_available_semaphores
                .iter()
                .for_each(|f| device.destroy_semaphore(*f, None));
            if self.command_handle.command_pool != vk::CommandPool::null() && !self.command_handle.command_buffers.is_empty() {
                device.free_command_buffers(self.command_handle.command_pool, &self.command_handle.command_buffers);
            }
            if self.command_handle.command_pool != vk::CommandPool::null() {
                device.destroy_command_pool(self.command_handle.command_pool, None);
            }
            self.command_handle.source_instance_buffer.destroy(&device);
            self.command_handle.main_camera_visbuffers.iter_mut().for_each(|v| v.destroy(&device));

            // Model Handle

            self.model_handle.vertex_buffer.destroy(&device);
            self.model_handle.index_buffer.destroy(&device);
            self.model_handle.loaded_meshes.clear();
            self.model_handle.uniform_buffers
                .iter_mut()
                .for_each(|b| b.destroy(&device));
            self.model_handle.dyn_model_matrix_buffer.destroy(&device);
            self.model_handle.static_model_matrix_buffer.destroy(&device);
            self.model_handle.mesh_uniform_buffer.destroy(&device);

            // Texture Handle

            self.texture_handle.loaded_textures.values().for_each(|t| {
                t.texture.destroy(device.clone());
            });
            self.texture_handle.samplers
                .iter()
                .for_each(|(_, s)| device.destroy_sampler(s.sampler, None));
            self.texture_handle.samplers.clear();
            self.texture_handle.loaded_textures.clear();
            self.texture_handle.available_texture_slots.clear();
            self.texture_handle.samplers.clear();
            self.texture_handle.available_sampler_slots.clear();
        }
    }

    pub fn add_instance(
        &mut self,
        mesh_asset_id: AssetId,
        texture_asset_id: AssetId,
        sampler_contents: SamplerContents,
        matrix: QuantizedModelMatrix,
        is_static: bool,
    ) -> Result<PerInstanceData> {
        // Potentially load model and texture
        let mesh_metadata_index = self.load_mesh(mesh_asset_id)?;
        self.load_texture(texture_asset_id, sampler_contents)?;

        // Get the mesh that the instance uses
        let mesh = self
            .model_handle
            .loaded_meshes
            .get(&mesh_asset_id)
            .unwrap()
            .clone();

        // Get bindless texture index
        let texture_index = self
            .get_texture_slot_index(texture_asset_id)
            .unwrap_or(0) as u16;

        // Get bindless sampler index
        let sampler_index = self
            .get_sampler_slot_index(sampler_contents)
            .unwrap_or(0) as u16;        

        // Create model matrix
        let model_matrix_info = self.create_model_matrix(matrix, is_static)?;

        // Add instance to the source instance buffer
        let new_instance = PerInstanceData {
            model_matrix_info,
            texture_index,
            sampler_index,
            mesh_metadata_index,
            mesh_asset_id: mesh_asset_id as u16,
        };
        self.command_handle.source_instance_buffer.add_item(&self.device_context, self.command_handle.command_pool, new_instance)?;

        Ok(new_instance)
    }

    pub fn remove_instance(
        &mut self,
        mesh_asset_id: AssetId,
        texture_asset_id: AssetId,
        sampler_contents: SamplerContents,
        model_matrix_info: u32,
    ) -> Result<()> {
        // Potentially unload model and texture

        self.unload_mesh(mesh_asset_id)?;
        self.unload_texture(texture_asset_id, sampler_contents)?;

        // Remove instance and model matrix

        let source_instances = self.command_handle.source_instance_buffer.get_buffer_items(&self.device_context, self.command_handle.command_pool, true)?;
        let instance_index = source_instances.iter().position(|instance| instance.model_matrix_info == model_matrix_info).expect(&format!("Instance with model matrix info of {} not found!", model_matrix_info));
        self.command_handle.source_instance_buffer.remove_item_at(&self.device_context, self.command_handle.command_pool, instance_index as u32)?;
        self.remove_model_matrix(VulkanRenderer::get_model_matrix_index(model_matrix_info), VulkanRenderer::is_model_matrix_static(model_matrix_info))?;
        Ok(())
    }

    // pub fn add_instance(
    //     &mut self,
    //     vertex_asset_id: AssetId,
    //     index_asset_id: AssetId,
    //     texture_asset_id: AssetId,
    //     sampler_contents: SamplerContents,
    //     model_matrix_info: u32,
    // ) -> Result<*const PerInstanceData> {
    //     // Potentially load model and texture
    //     self.load_model(
    //         vertex_asset_id,
    //         index_asset_id,
    //     )?;
    //     self.load_texture(texture_asset_id, sampler_contents)?;

    //     // Get model that the instance uses
    //     let model = self
    //         .model_handle
    //         .loaded_models
    //         .get(&(vertex_asset_id, index_asset_id))
    //         .unwrap()
    //         .clone();

    //     // Get bindless texture index
    //     let tex_index = self
    //         .get_texture_slot_index(texture_asset_id)
    //         .unwrap_or(0);

    //     // Get bindless sampler index
    //     let sampler_index = self
    //         .get_sampler_slot_index(sampler_contents)
    //         .unwrap_or(0);

    //     unsafe {
    //         let mut affected_draw_data: Vec<*mut IndirectDrawData> = self
    //             .model_handle
    //             .loaded_models
    //             .iter()
    //             .filter(|&(_, m)| {
    //                 m.indirect_draw_data_ptr.read().first_instance
    //                     > model.indirect_draw_data_ptr.read().first_instance
    //             })
    //             .map(|(_, m)| m.indirect_draw_data_ptr)
    //             .collect();

    //         affected_draw_data
    //             .sort_unstable_by(|a, b| a.read().first_instance.cmp(&b.read().first_instance));

    //         // Add one to the instance offset for those found in the buffer after the new instance
    //         // and continuously swap beginnings of instance buffer sections to make room for new instance
    //         if affected_draw_data.len() > 0 {
    //             let mut saved_instance: PerInstanceData = self
    //                 .command_handle
    //                 .instance_buffer
    //                 .mapped
    //                 .add(affected_draw_data[0].read().first_instance as usize)
    //                 .read();

    //             for draw_data in affected_draw_data {
    //                 let draw_data = draw_data.as_mut().unwrap();
    //                 let next_instance_ptr = self
    //                     .command_handle
    //                     .instance_buffer
    //                     .mapped
    //                     .add((draw_data.first_instance + draw_data.instance_count) as usize)
    //                     .cast_mut();

    //                 let next_instance = next_instance_ptr.read();
    //                 *next_instance_ptr = saved_instance;
    //                 saved_instance = next_instance;

    //                 draw_data.first_instance += 1;
    //             }
    //         }

    //         // Add instance to instance buffer
    //         let new_instance = PerInstanceData {
    //             model_matrix_info: model_matrix_info,
    //             texture_index: tex_index,
    //             sampler_index: sampler_index,
    //             padding: 0,
    //         };

    //         let draw_data = model.indirect_draw_data_ptr.as_mut().unwrap();

    //         let new_instance_ptr = self
    //             .command_handle
    //             .instance_buffer
    //             .mapped
    //             .add((draw_data.first_instance + draw_data.instance_count) as usize)
    //             .cast_mut();

    //         *new_instance_ptr = new_instance;
    //         draw_data.instance_count += 1;

    //         Ok(new_instance_ptr)
    //     }
    // }

    // pub fn remove_instance(
    //     &mut self,
    //     vertex_asset_id: AssetId,
    //     index_asset_id: AssetId,
    //     texture_asset_id: AssetId,
    //     sampler_contents: SamplerContents,
    //     instance: *const PerInstanceData,
    // ) -> Result<()> {
    //     let temp_instance_model_info = unsafe { instance.read().model_matrix_info };

    //     // Potentially unload model and texture
    //     self.unload_model(vertex_asset_id, index_asset_id)?;
    //     self.unload_texture(texture_asset_id, sampler_contents)?;

    //     // Get model that the instance uses
    //     let model = self
    //         .model_handle
    //         .loaded_models
    //         .get(&(vertex_asset_id, index_asset_id))
    //         .unwrap()
    //         .clone();

    //     let draw_data = unsafe { model.indirect_draw_data_ptr.as_mut().unwrap() };

    //     unsafe {
    //         // Replace the removed instance with the instance at the end of the instance buffer section
    //         draw_data.instance_count -= 1;
    //         *instance.cast_mut() = self
    //             .command_handle
    //             .instance_buffer
    //             .mapped
    //             .add((draw_data.first_instance + draw_data.instance_count) as usize)
    //             .read();

    //         // Get draw datas affected by this removal
    //         let mut affected_draw_data: Vec<*const IndirectDrawData> = self
    //             .model_handle
    //             .loaded_models
    //             .iter()
    //             .filter(|&(_, m)| {
    //                 m.indirect_draw_data_ptr.read().first_instance > draw_data.first_instance
    //             })
    //             .map(|(_, m)| m.indirect_draw_data_ptr.cast_const())
    //             .collect();

    //         affected_draw_data
    //             .sort_unstable_by(|a, b| a.read().first_instance.cmp(&b.read().first_instance));

    //         // Remove one from the instance offset for those found in the buffer after the new instance
    //         // and continuously overwrite ends of instance buffer sections with the next ends to cover the empty instance
    //         let mut instance_to_overwrite: *mut PerInstanceData = self
    //             .command_handle
    //             .instance_buffer
    //             .mapped
    //             .add((draw_data.first_instance + draw_data.instance_count) as usize)
    //             .cast_mut();

    //         for draw_data in affected_draw_data {
    //             let draw_data = draw_data.cast_mut().as_mut().unwrap();
    //             draw_data.first_instance -= 1;

    //             let this_instance = self
    //                 .command_handle
    //                 .instance_buffer
    //                 .mapped
    //                 .add((draw_data.first_instance + draw_data.instance_count) as usize)
    //                 .cast_mut();

    //             *instance_to_overwrite = this_instance.read();
    //             *this_instance = PerInstanceData {
    //                 model_matrix_info: u32::MAX,
    //                 texture_index: u32::MAX,
    //                 sampler_index: u32::MAX,
    //                 padding: u32::MAX,
    //             };
    //             instance_to_overwrite = this_instance;
    //         }

    //         // Unload model if there are no more instances using it
    //         if draw_data.instance_count == 0 {
    //             let mut last_draw_data: *const IndirectDrawData = std::ptr::null();
    //             for i in (0..self.command_handle.indirect_draw_buffer.capacity).rev() {
    //                 let draw_data = self.command_handle.indirect_draw_buffer.mapped.add(i);
    //                 if draw_data.read().instance_count > 0 {
    //                     last_draw_data = draw_data;
    //                     break;
    //                 }
    //             }

    //             self.model_handle
    //                 .loaded_models
    //                 .iter_mut()
    //                 .find(|(_, m)| m.indirect_draw_data_ptr == last_draw_data.cast_mut())
    //                 .map(|(_, m)| m.indirect_draw_data_ptr = model.indirect_draw_data_ptr);

    //             *model.indirect_draw_data_ptr = last_draw_data.read();
    //             *last_draw_data.cast_mut() = IndirectDrawData::zeroed();

    //             self.model_handle
    //                 .loaded_models
    //                 .remove(&(vertex_asset_id, index_asset_id));
    //         }
    //     }

    //     Ok(())
    // }

    pub fn create_model_matrix(
        &mut self,
        matrix: QuantizedModelMatrix,
        is_static: bool,
    ) -> Result<u32> {
        if is_static {
            let prev_buffer = self.model_handle.static_model_matrix_buffer.buffer;
            let chosen_index = self.model_handle.static_model_matrix_buffer.add_item(
                &self.device_context,
                self.command_handle.command_pool,
                matrix,
            )?;
            let new_buffer = self.model_handle.static_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                self.update_model_matrix_buffer_descriptors()?;
            }
            return Ok(chosen_index);
        } else {
            let prev_buffer = self.model_handle.dyn_model_matrix_buffer.buffer;
            let chosen_index = self.model_handle.dyn_model_matrix_buffer.add_item(
                &self.device_context,
                self.command_handle.command_pool,
                matrix,
            )?;
            let new_buffer = self.model_handle.dyn_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                self.update_model_matrix_buffer_descriptors()?;
            }

            let mut info = chosen_index & 0x7FFFFFFF;
            info |= (is_static as u32) << 31;
            Ok(info)
        }
    }

    pub fn remove_model_matrix(&mut self, model_matrix_index: u32, is_static: bool) -> Result<()> {
        if is_static {
            let prev_buffer = self.model_handle.static_model_matrix_buffer.buffer;
            self.model_handle
                .static_model_matrix_buffer
                .remove_item_at(&self.device_context, self.command_handle.command_pool, model_matrix_index)?;
            let new_buffer = self.model_handle.static_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                self.update_model_matrix_buffer_descriptors()?;
            }
        } else {
            let prev_buffer = self.model_handle.dyn_model_matrix_buffer.buffer;
            self.model_handle.dyn_model_matrix_buffer.remove_item_at(
                &self.device_context,
                self.command_handle.command_pool,
                model_matrix_index,
            )?;
            let new_buffer = self.model_handle.dyn_model_matrix_buffer.buffer;

            if prev_buffer != new_buffer {
                self.update_model_matrix_buffer_descriptors()?;
            }
        }

        Ok(())
    }

    pub fn is_model_matrix_static(model_matrix_info: u32) -> bool {
        model_matrix_info & 0x80000000 > 0
    }    

    pub fn get_model_matrix_index(model_matrix_info: u32) -> u32 {
        model_matrix_info & 0x7FFFFFFF
    }

    pub fn get_model_matrix(
        &self,
        model_matrix_index: u32,
        is_static: bool,
    ) -> QuantizedModelMatrix {
        if is_static {
            unsafe {
                self.model_handle
                    .static_model_matrix_buffer
                    .mapped
                    .add(model_matrix_index as usize)
                    .read()
            }
        } else {
            unsafe {
                self.model_handle
                    .dyn_model_matrix_buffer
                    .mapped
                    .add(model_matrix_index as usize)
                    .read()
            }
        }
    }

    pub fn get_model_matrix_mut(
        &self,
        model_matrix_index: u32,
        is_static: bool,
    ) -> &mut QuantizedModelMatrix {
        if is_static {
            unsafe {
                self.model_handle
                    .static_model_matrix_buffer
                    .mapped
                    .add(model_matrix_index as usize)
                    .cast_mut()
                    .as_mut()
                    .unwrap()
            }
        } else {
            unsafe {
                self.model_handle
                    .dyn_model_matrix_buffer
                    .mapped
                    .add(model_matrix_index as usize)
                    .cast_mut()
                    .as_mut()
                    .unwrap()
            }
        }
    }

    pub fn set_model_matrix(
        &mut self,
        model_matrix_index: u32,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
        is_static: bool,
    ) -> Result<()> {
        let buffer_ptr = if is_static {
            self.model_handle.static_model_matrix_buffer.mapped
        } else {
            self.model_handle.dyn_model_matrix_buffer.mapped
        };
        let model_matrix = unsafe {
            buffer_ptr
                .add(model_matrix_index as usize)
                .cast_mut()
                .as_mut()
                .unwrap()
        };

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

    /// Returns mesh metadata index inside uniform buffer
    pub fn load_mesh(&mut self, mesh_asset_id: AssetId) -> Result<u16> {
        if mesh_asset_id == AssetId::None {
            return Ok(u16::MAX)
        }

        // Do not load mesh if it is already loaded

        if let Some(mesh) = self.model_handle.loaded_meshes.get_mut(&mesh_asset_id) {
            mesh.usage_count += 1;
            return Ok(u16::MAX)
        }

        let asset = get_asset_from_id(mesh_asset_id);

        // Get vertex and index counts

        let vertex_count0 = u32::from_be_bytes(asset.0[0..4].try_into().unwrap()) as usize;
        let index_count0 = u32::from_be_bytes(asset.0[4..8].try_into().unwrap()) as usize;
        let vertex_byte_count0 = u32::from_be_bytes(asset.0[8..12].try_into().unwrap()) as usize;
        let index_byte_count0 = u32::from_be_bytes(asset.0[12..16].try_into().unwrap()) as usize;
        let vertex_count1 = u32::from_be_bytes(asset.0[16..20].try_into().unwrap()) as usize;
        let index_count1 = u32::from_be_bytes(asset.0[20..24].try_into().unwrap()) as usize;
        let vertex_byte_count1 = u32::from_be_bytes(asset.0[24..28].try_into().unwrap()) as usize;
        let index_byte_count1 = u32::from_be_bytes(asset.0[28..32].try_into().unwrap()) as usize;
        let vertex_count2 = u32::from_be_bytes(asset.0[32..36].try_into().unwrap()) as usize;
        let index_count2 = u32::from_be_bytes(asset.0[36..40].try_into().unwrap()) as usize;
        let vertex_byte_count2 = u32::from_be_bytes(asset.0[40..44].try_into().unwrap()) as usize;
        let index_byte_count2 = u32::from_be_bytes(asset.0[44..48].try_into().unwrap()) as usize;
        let vertex_count3 = u32::from_be_bytes(asset.0[48..52].try_into().unwrap()) as usize;
        let index_count3 = u32::from_be_bytes(asset.0[52..56].try_into().unwrap()) as usize;
        let vertex_byte_count3 = u32::from_be_bytes(asset.0[56..60].try_into().unwrap()) as usize;
        let index_byte_count3 = u32::from_be_bytes(asset.0[60..64].try_into().unwrap()) as usize;

        let total_vertex_byte_count = vertex_byte_count0 + vertex_byte_count1 + vertex_byte_count2 + vertex_byte_count3;

        // Get AABB

        let min_aabb: Vec3 = Vec3::new(
            f32::from_be_bytes(asset.0[64..68].try_into().unwrap()),
            f32::from_be_bytes(asset.0[68..72].try_into().unwrap()),
            f32::from_be_bytes(asset.0[72..76].try_into().unwrap()),
        );
        let max_aabb: Vec3 = Vec3::new(
            f32::from_be_bytes(asset.0[76..80].try_into().unwrap()),
            f32::from_be_bytes(asset.0[80..84].try_into().unwrap()),
            f32::from_be_bytes(asset.0[84..88].try_into().unwrap()),
        );

        // Get lod data

        let lod1_percentage = f32::from_be_bytes(asset.0[88..92].try_into().unwrap());
        let lod2_percentage = f32::from_be_bytes(asset.0[92..96].try_into().unwrap());
        let lod3_percentage = f32::from_be_bytes(asset.0[96..100].try_into().unwrap());
        let cull_percentage = f32::from_be_bytes(asset.0[100..104].try_into().unwrap());
        
        // Get vertex and index bytes

        let mut vertex_offset = 104;
        let mut index_offset = 104 + total_vertex_byte_count;

        let vertex_bytes0: &[u8] = &asset.0[vertex_offset..(vertex_offset + vertex_byte_count0)];
        let index_bytes0: &[u8] = &asset.0[index_offset..(index_offset + index_byte_count0)];

        vertex_offset += vertex_byte_count0;
        index_offset += index_byte_count0;

        let vertex_bytes1: &[u8] = &asset.0[vertex_offset..(vertex_offset + vertex_byte_count1)];
        let index_bytes1: &[u8] = &asset.0[index_offset..(index_offset + index_byte_count1)];

        vertex_offset += vertex_byte_count1;
        index_offset += index_byte_count1;

        let vertex_bytes2: &[u8] = &asset.0[vertex_offset..(vertex_offset + vertex_byte_count2)];
        let index_bytes2: &[u8] = &asset.0[index_offset..(index_offset + index_byte_count2)];

        vertex_offset += vertex_byte_count2;
        index_offset += index_byte_count2;

        let vertex_bytes3: &[u8] = &asset.0[vertex_offset..(vertex_offset + vertex_byte_count3)];
        let index_bytes3: &[u8] = &asset.0[index_offset..(index_offset + index_byte_count3)];

        // Decode vertices and indices

        let mut vertices0: Vec<QuantizedVertex> = match meshopt::decode_vertex_buffer(vertex_bytes0, vertex_count0) {
            Ok(bytes) => bytes,
            Err(_) => return Err(anyhow!("Failed to decode vertex buffer 0")),
        };
        let mut indices0: Vec<u32> = match meshopt::decode_index_buffer(index_bytes0, index_count0) {
            Ok(indices) => indices,
            Err(_) => return Err(anyhow!("Failed to decode index buffer 0")),
        };

        let vertices1: Vec<QuantizedVertex> = if vertex_count1 > 0 {
            match meshopt::decode_vertex_buffer(vertex_bytes1, vertex_count1) {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("Failed to decode vertex buffer 1")),
            }
        } else {
            Vec::new()
        };
        let indices1: Vec<u32> = if index_count1 > 0 {
            match meshopt::decode_index_buffer(index_bytes1, index_count1) {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("Failed to decode index buffer 1")),
            }
        } else {
            Vec::new()
        };

        let vertices2: Vec<QuantizedVertex> = if vertex_count2 > 0 {
            match meshopt::decode_vertex_buffer(vertex_bytes2, vertex_count2) {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("Failed to decode vertex buffer 2")),
            }
        } else {
            Vec::new()
        };
        let indices2: Vec<u32> = if index_count2 > 0 {
            match meshopt::decode_index_buffer(index_bytes2, index_count2) {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("Failed to decode index buffer 2")),
            }
        } else {
            Vec::new()
        };

        let vertices3: Vec<QuantizedVertex> = if vertex_count3 > 0 {
            match meshopt::decode_vertex_buffer(vertex_bytes3, vertex_count3) {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("Failed to decode vertex buffer 3")),
            }
        } else {
            Vec::new()
        };
        let indices3: Vec<u32> = if index_count3 > 0 {
            match meshopt::decode_index_buffer(index_bytes3, index_count3) {
                Ok(bytes) => bytes,
                Err(_) => return Err(anyhow!("Failed to decode index buffer 3")),
            }
        } else {
            Vec::new()
        };

        // Finish loading

        vertices0.extend(vertices1);
        vertices0.extend(vertices2);
        vertices0.extend(vertices3);
        indices0.extend(indices1);
        indices0.extend(indices2);
        indices0.extend(indices3);

        let prev_vertex_count = self.model_handle.vertex_buffer.element_count;
        let prev_index_count = self.model_handle.index_buffer.element_count;

        self.model_handle.vertex_buffer.add_items(
            &self.device_context,
            self.command_handle.command_pool,
            vertices0,
        )?;

        self.model_handle.index_buffer.add_items(
            &self.device_context,
            self.command_handle.command_pool,
            indices0,
        )?;

        let mesh_metadata = MeshMetadata::new(min_aabb, max_aabb, lod1_percentage, lod2_percentage, lod3_percentage, cull_percentage);
        let mesh = Mesh {
            mesh_asset_id,
            metadata: mesh_metadata,
            usage_count: 1,
            vertex_offset: prev_vertex_count,
            index_offset: prev_index_count,
            lod0_vertex_length: vertex_count0 as u32,
            lod0_index_length: index_count0 as u32,
            lod1_vertex_length: vertex_count1 as u32,
            lod1_index_length: index_count1 as u32,
            lod2_vertex_length: vertex_count2 as u32,
            lod2_index_length: index_count2 as u32,
            lod3_vertex_length: vertex_count3 as u32,
            lod3_index_length: index_count3 as u32,
        };

        self.model_handle.loaded_meshes.insert(mesh_asset_id, mesh);

        let (mesh_buffer, mesh_buffer_memory, mesh_buffer_mapped) = self.model_handle.mesh_uniform_buffer.get_buffer_parts(&self.device_context, self.command_handle.command_pool);
        let mesh_buffer_layout = unsafe { mesh_buffer_mapped.cast_mut().as_mut().unwrap() };
        
        let mut mesh_metadata_index = u16::MAX;
        for i in 0..2048 {
            if mesh_buffer_layout.0[i] == mesh_metadata {
                mesh_metadata_index = i as u16;
                break;
            }
        }

        if mesh_metadata_index == u16::MAX {
            for i in 0..2048 {
                if mesh_buffer_layout.0[i] == MeshMetadata::default() {
                    mesh_buffer_layout.0[i] = mesh_metadata;
                    mesh_metadata_index = i as u16;
                    break;
                }
            }
        }

        // Finalize and Cleanup
        unsafe {
            if !self.model_handle.mesh_uniform_buffer.is_host_visible {
                Buffer::<MeshBufferLayout>::copy_buffer(&self.device_context, self.command_handle.command_pool, mesh_buffer, self.model_handle.mesh_uniform_buffer.buffer, 1u64)?;
                self.device_context.device.destroy_buffer(mesh_buffer, None);
                self.device_context.device.unmap_memory(mesh_buffer_memory);
                self.device_context.device.free_memory(mesh_buffer_memory, None);
            }
        }

        Ok(mesh_metadata_index)
    }

    pub fn unload_mesh(&mut self, mesh_asset_id: AssetId) -> Result<()> {
        // Do not unload mesh if it is not loaded

        if mesh_asset_id == AssetId::None || !self.model_handle.loaded_meshes.contains_key(&mesh_asset_id) {
            return Ok(())
        }

        self.model_handle.loaded_meshes.get_mut(&mesh_asset_id).unwrap().usage_count -= 1;

        let unloading_mesh = self
            .model_handle
            .loaded_meshes
            .get(&mesh_asset_id)
            .unwrap()
            .clone();

        if unloading_mesh.usage_count == 0 {
            let unloaded_vertex_count = unloading_mesh.lod0_vertex_length + unloading_mesh.lod1_vertex_length + unloading_mesh.lod2_vertex_length + unloading_mesh.lod3_vertex_length;
            let unloaded_index_count = unloading_mesh.lod0_index_length + unloading_mesh.lod1_index_length + unloading_mesh.lod2_index_length + unloading_mesh.lod3_index_length;

            self.model_handle.vertex_buffer.remove_items(
                &self.device_context,
                self.command_handle.command_pool,
                unloading_mesh.vertex_offset,
                unloading_mesh.vertex_offset + unloaded_vertex_count,
            )?;
            self.model_handle.index_buffer.remove_items(
                &self.device_context,
                self.command_handle.command_pool,
                unloading_mesh.index_offset,
                unloading_mesh.index_offset + unloaded_index_count,
            )?;

            // Update other model offsets
            self.model_handle
                .loaded_meshes
                .values_mut()
                .filter(|m| m.vertex_offset > unloading_mesh.vertex_offset)
                .for_each(|m| {
                    m.vertex_offset -= unloaded_vertex_count;
                    m.index_offset -= unloaded_index_count;
                });

            self.model_handle.loaded_meshes.remove(&mesh_asset_id);
        }

        Ok(())
    }

    pub fn load_texture(&mut self, texture_asset_id: AssetId, sampler_contents: SamplerContents) -> Result<()> {
        if texture_asset_id == AssetId::None {
            return Ok(());
        }

        if let Some(texture) = self.texture_handle.loaded_textures.get_mut(&texture_asset_id) {
            texture.instance_count += 1;
            return Ok(());
        }

        let device_context = self.device_context.clone();

        // Load

        let texture = {
            let mut texture =
                ktx2_rw::Ktx2Texture::from_memory(get_asset_from_id(texture_asset_id).0)?;

            // Try BC7 first, fall back to ASTC 4x4 if not supported
            let transcode_format = if is_image_format_supported(
                device_context.clone().instance,
                device_context.physical_device,
                vk::Format::BC7_SRGB_BLOCK,
            ) {
                info!("Using BC7 format for texture transcoding");
                ktx2_rw::TranscodeFormat::Bc7Rgba
            } else if is_image_format_supported(
                device_context.clone().instance,
                device_context.physical_device,
                vk::Format::ASTC_4X4_SRGB_BLOCK,
            ) {
                info!("BC7 not supported, falling back to ASTC 4x4 for texture transcoding");
                ktx2_rw::TranscodeFormat::Astc_4x4_Rgba
            } else {
                return Err(anyhow!(
                    "Neither BC7 nor ASTC 4x4 compression formats are supported"
                ));
            };

            texture
                .transcode_basis(transcode_format)
                .expect("Failed to transcode texture image format");
            texture
        };

        let format = vk::Format::from_raw(texture.vk_format().as_raw() as i32);
        let pixel_data = texture.get_image_data(0, 0, 0).unwrap();
        let mipmap_levels = texture.levels();

        // Update sampler contents mipmap levels
        let mut sampler_contents = sampler_contents;
        sampler_contents.mipmap_levels = mipmap_levels;

        // Calculate total size for all mip levels and collect per-level data
        let mut mip_sizes: Vec<usize> = Vec::with_capacity(mipmap_levels as usize);
        let mut total_size: u64 = 0;
        for level in 0..mipmap_levels {
            let mip_pixel_data = texture.get_image_data(level, 0, 0).unwrap();
            mip_sizes.push(mip_pixel_data.len());
            total_size += mip_pixel_data.len() as u64;
        }

        // Create (staging)

        let (staging_buffer, staging_buffer_memory, staging_buffer_mapped) = Buffer::<*const c_void>::create_buffer(
            &device_context,
            total_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;

        // Copy (staging)

        unsafe {
            // Copy each mip level into the staging buffer at the correct offset
            let mut offset: usize = 0;
            for level in 0..mipmap_levels as usize {
                let mip_pixel_data = texture.get_image_data(level as u32, 0, 0).unwrap();
                memcpy(
                    mip_pixel_data.as_ptr(),
                    staging_buffer_mapped.add(offset).cast(),
                    mip_pixel_data.len(),
                );
                offset += mip_pixel_data.len();
            }

            device_context.clone().device.unmap_memory(staging_buffer_memory);
        }

        // Create (Image)

        let (texture_image, texture_image_memory) = create_image(
            device_context.clone(),
            texture.width(),
            texture.height(),
            mipmap_levels,
            vk::SampleCountFlags::_1,
            format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        // Transition + Copy (image)

        transition_image_layout(
            device_context.clone(),
            self.command_handle.command_pool,
            texture_image,
            format,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            mipmap_levels,
        )?;

        // Copy each mip level from the staging buffer into the corresponding image mip level
        let command_buffer = begin_single_time_commands(self.command_handle.command_pool, device_context.clone().device)?;

        let mut buffer_offset: u64 = 0;
        let mut regions: Vec<vk::BufferImageCopy> = Vec::with_capacity(mipmap_levels as usize);
        for level in 0..mipmap_levels {
            let mip_width = (texture.width() >> level).max(1);
            let mip_height = (texture.height() >> level).max(1);

            let subresource = vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(level)
                .base_array_layer(0)
                .layer_count(1)
                .build();

            let region = vk::BufferImageCopy::builder()
                .buffer_offset(buffer_offset)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(subresource)
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: mip_width,
                    height: mip_height,
                    depth: 1,
                })
                .build();

            regions.push(region);

            buffer_offset += mip_sizes[level as usize] as u64;
        }

        unsafe {
            device_context.device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer,
                texture_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            );
        }

        end_single_time_commands(self.command_handle.command_pool, command_buffer, device_context.device.clone(), device_context.clone().device_queue_handle)?;

        transition_image_layout(
            device_context.clone(),
            self.command_handle.command_pool,
            texture_image,
            format,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            mipmap_levels,
        )?;

        // Cleanup

        unsafe {
            device_context.device.destroy_buffer(staging_buffer, None);
            device_context.device.free_memory(staging_buffer_memory, None);
        }

        // Create view
        let image_view = create_image_view(
            device_context.clone().device,
            texture_image,
            format,
            vk::ImageAspectFlags::COLOR,
            mipmap_levels,
        )?;

        // Add one to the usage count if a sampler was found that already has required specifications, otherwise create sampler
        let sampler = if let Some(usage) = self.texture_handle.samplers.get_mut(&sampler_contents) {
            usage.instance_count += 1;
            self.texture_handle.samplers[&sampler_contents].sampler
        } else {
            let sampler = create_sampler(device_context.clone().device, sampler_contents);

            // Get sampler slot index
            let slot_index: u32 = if self.texture_handle.available_sampler_slots.len() > 0 {
                self.texture_handle.available_sampler_slots.pop().unwrap()
            } else {
                self.texture_handle.samplers.len() as u32
            };
            let sampler_usage = SamplerUsage {
                slot_index,
                sampler,
                instance_count: 1,
            };
            self.texture_handle.samplers.insert(sampler_contents, sampler_usage);
            update_bindless_sampler(
                &device_context,
                &self.render_pipeline_handle.descriptor_handle.descriptor_sets,
                &self.texture_handle,
                slot_index,
                sampler,
            )?;
            sampler
        };

        // Add texture to array of textures
        let slot_index: u32 = if self.texture_handle.available_texture_slots.len() > 0 {
            self.texture_handle.available_texture_slots.pop().unwrap()
        } else {
            self.texture_handle.loaded_textures.len() as u32
        };
        self.texture_handle.loaded_textures.insert(
            texture_asset_id,
            TextureUsage {
                texture: Texture {
                    image: texture_image,
                    image_memory: texture_image_memory,
                    image_view,
                },
                slot_index,
                instance_count: 1,
            },
        );

        // Update bindless descriptor
        update_bindless_texture(
            &device_context,
            &self.render_pipeline_handle.descriptor_handle.descriptor_sets,
            &self.texture_handle,slot_index, image_view
        )?;

        Ok(())
    }

    pub fn unload_texture(
        &mut self,
        texture_asset_id: AssetId,
        sampler_contents: SamplerContents,
    ) -> Result<()> {
        if texture_asset_id == AssetId::None {
            return Ok(());
        }

        let (unloading_texture, fully_unloaded) =
            if let Some(texture) = self.texture_handle.loaded_textures.get_mut(&texture_asset_id) {
                texture.instance_count -= 1;
                (*texture, texture.instance_count == 0)
            } else {
                return Err(anyhow!("Texture not found"));
            };

        if fully_unloaded {
            self.texture_handle.loaded_textures.remove(&texture_asset_id);
            self.texture_handle.available_texture_slots
                .push(unloading_texture.slot_index);
            unsafe {
                self.device_context
                    .device
                    .destroy_image_view(unloading_texture.texture.image_view, None);
                self.device_context.device.destroy_image(unloading_texture.texture.image, None);
                self.device_context.device.free_memory(unloading_texture.texture.image_memory, None);

                // Unload sampler if no more textures use this sampler
                let sampler = self.texture_handle.samplers[&sampler_contents].sampler.clone();
                let slot_index = self.texture_handle.samplers[&sampler_contents].slot_index;
                let should_unload_sampler: bool =
                    if let Some(usage) = self.texture_handle.samplers.get_mut(&sampler_contents) {
                        usage.instance_count -= 1;
                        usage.instance_count == 0
                    } else {
                        false
                    };

                if should_unload_sampler {
                    self.device_context.device.destroy_sampler(sampler, None);
                    self.texture_handle.samplers.remove(&sampler_contents);
                    self.texture_handle.available_sampler_slots.push(slot_index);
                }
            }
        }

        Ok(())
    }

    pub fn get_texture_slot_index(&self, texture_asset_id: AssetId) -> Option<u32> {
        self.texture_handle.loaded_textures
            .get(&texture_asset_id)
            .map(|texture| texture.slot_index)
    }

    pub fn get_sampler_slot_index(&self, sampler_contents: SamplerContents) -> Option<u32> {
        self.texture_handle.samplers.get(&sampler_contents).map(|s| s.slot_index)
    }

    /// Recreates the swapchain for the Vulkan app
    #[rustfmt::skip]
    fn recreate_swapchain(&mut self) -> Result<()> {
        unsafe {
            let size = self.present_handle.window_handle.window.inner_size();
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            self.device_context.device.device_wait_idle()?;
            self.destroy_swapchain();
            
            // Update presentation
            self.present_handle.swapchain_handle = create_swapchain(&self.device_context, &self.present_handle.window_handle.window, self.present_handle.window_handle.surface)?;
            self.present_handle.color_texture = create_color_texture(&self.device_context, &self.present_handle.swapchain_handle, self.present_handle.msaa_samples)?;
            self.present_handle.depth_texture = create_depth_texture(&self.device_context, &self.present_handle.swapchain_handle, self.present_handle.msaa_samples)?;
            
            // Update ubos
            self.model_handle.uniform_buffers = create_uniform_buffers(&self.device_context, self.command_handle.command_pool);

            // Update render pipeline
            self.render_pipeline_handle.base_render_pass = create_base_render_pass(&self.device_context, &self.present_handle.swapchain_handle, self.present_handle.msaa_samples)?;
            self.render_pipeline_handle.descriptor_handle.descriptor_pool = create_descriptor_pool(self.device_context.device.clone())?;
            (self.render_pipeline_handle.pipeline, self.render_pipeline_handle.pipeline_layout) = create_pipeline(
                &self.present_handle.swapchain_handle,
                self.present_handle.msaa_samples,
                self.render_pipeline_handle.descriptor_handle.descriptor_set_layout,
                self.render_pipeline_handle.base_render_pass,
                &self.device_context,
            )?;
            self.render_pipeline_handle.framebuffers = create_framebuffers(
                &self.present_handle.swapchain_handle,
                self.present_handle.color_texture,
                self.present_handle.depth_texture,
                self.render_pipeline_handle.base_render_pass,
                self.device_context.device.clone()
            )?;
            self.render_pipeline_handle.descriptor_handle.descriptor_sets = create_descriptor_sets(
                &self.device_context,
                &self.model_handle,
                &self.command_handle.main_camera_visbuffers,
                &self.texture_handle,
                self.render_pipeline_handle.descriptor_handle.descriptor_set_layout,
                self.render_pipeline_handle.descriptor_handle.descriptor_pool)?;

            // Update command buffers
            self.command_handle.command_buffers = create_command_buffers(self.device_context.device.clone(), self.command_handle.command_pool, self.render_pipeline_handle.framebuffers.len())?;
            self.command_handle.sync_handle.images_in_flight.resize(self.present_handle.swapchain_handle.images.len(), vk::Fence::null());
            
            Ok(())
        }
    }

    /// Destroys the parts of our Vulkan app related to the swapchain
    #[rustfmt::skip]
    fn destroy_swapchain(&mut self) {
        unsafe {
            let device = self.device_context.device.clone();
            if self.command_handle.command_pool != vk::CommandPool::null() && !self.command_handle.command_buffers.is_empty() {
                device.free_command_buffers(self.command_handle.command_pool, &self.command_handle.command_buffers);
            }
            device.destroy_descriptor_pool(self.render_pipeline_handle.descriptor_handle.descriptor_pool, None);
            self.present_handle.depth_texture.destroy(device.clone());
            self.present_handle.color_texture.destroy(device.clone());
            self.render_pipeline_handle.framebuffers.iter().for_each(|f| device.destroy_framebuffer(*f, None));
            device.destroy_pipeline(self.render_pipeline_handle.pipeline, None);
            device.destroy_pipeline_layout(self.render_pipeline_handle.pipeline_layout, None);
            device.destroy_render_pass(self.render_pipeline_handle.base_render_pass, None);
            self.present_handle.swapchain_handle.image_views.iter().for_each(|v| device.destroy_image_view(*v, None));
            device.destroy_swapchain_khr(self.present_handle.swapchain_handle.swapchain, None);
            self.model_handle.uniform_buffers.iter_mut().for_each(|b| b.destroy(&device));
            self.model_handle.uniform_buffers.clear();
        }
    }

    /// Renders a frame for the Vulkan app
    pub fn render(&mut self) -> Result<()> {
        unsafe {
            //let start = Instant::now();

            let device = self.device_context.device.clone();

            let size = self
                .present_handle
                .window_handle
                .window
                .inner_size();
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            let current_frame = self.command_handle.sync_handle.current_frame;
            let max_frames_in_flight = self.command_handle.sync_handle.max_frames_in_flight;

            let in_flight_fence =
                self.command_handle.sync_handle.in_flight_fences[current_frame];

            device.wait_for_fences(&[in_flight_fence], true, u64::MAX)?;

            //println!("wait before swap took: {:?}", start.elapsed());
            //let start = Instant::now();

            let result = device.acquire_next_image_khr(
                self.present_handle.swapchain_handle.swapchain,
                u64::MAX,
                self.command_handle.sync_handle.image_available_semaphores[current_frame],
                vk::Fence::null(),
            );

            let image_index = match result {
                Ok((image_index, _)) => image_index as usize,
                Err(vk::ErrorCode::OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain()?;
                    return Ok(());
                }
                Err(e) => return Err(anyhow!(e)),
            };

            let image_in_flight = self.command_handle.sync_handle.images_in_flight[image_index];
            if !image_in_flight.is_null() {
                device.wait_for_fences(&[image_in_flight], true, u64::MAX)?;
            }

            self.command_handle.sync_handle.images_in_flight[image_index] = in_flight_fence;

            //println!("Swapchain took: {:?}", start.elapsed());
            //let start = Instant::now();

            self.update_uniform_buffer(current_frame)?;

            //println!("Uniform buffer took: {:?}", start.elapsed());
            //let start = Instant::now();

            let command_buffer = self.command_handle.command_buffers[image_index];
            let indirect_draw_buffer = &self.command_handle.main_camera_visbuffers[current_frame].indirect_draw_buffer;

            // Culling Pass



            // Commands

            device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

            let info = vk::CommandBufferBeginInfo::builder();
            device.begin_command_buffer(command_buffer, &info)?;

            //println!("Draw, instance, and command buffer update took: {:?}", start.elapsed());
            //let start = Instant::now();

            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(self.present_handle.swapchain_handle.extent);

            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 1.0, 1.0],
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
                .render_pass(self.render_pipeline_handle.base_render_pass)
                .framebuffer(self.render_pipeline_handle.framebuffers[image_index])
                .render_area(render_area)
                .clear_values(clear_values);

            //println!("Clearing took: {:?}", start.elapsed());
            //let start = Instant::now();

            device.cmd_begin_render_pass(command_buffer, &info, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.render_pipeline_handle.pipeline,
            );

            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.model_handle.vertex_buffer.buffer],
                &[0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.model_handle.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.render_pipeline_handle.pipeline_layout,
                0,
                &[self.render_pipeline_handle.descriptor_handle.descriptor_sets[current_frame]],
                &[],
            );

            device.cmd_draw_indexed_indirect(
                command_buffer,
                indirect_draw_buffer.buffer,
                0,
                indirect_draw_buffer.element_capacity as u32,
                size_of::<IndirectDrawData>() as u32,
            );

            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;

            //println!("Indirect drawing took: {:?}", start.elapsed());
            //let start = Instant::now();

            let wait_semaphores = &[self.command_handle.sync_handle.image_available_semaphores[current_frame]];
            let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let command_buffers = &[command_buffer];
            let signal_semaphores = &[self.command_handle.sync_handle.render_finished_semaphores[image_index]];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(wait_stages)
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            device.reset_fences(&[in_flight_fence])?;

            device.queue_submit(self.device_context.device_queue_handle.graphics_queue, &[submit_info], in_flight_fence)?;

            //println!("Sem and fences took: {:?}", start.elapsed());
            //let start = Instant::now();

            let swapchains = &[self.present_handle.swapchain_handle.swapchain];
            let image_indices = &[image_index as u32];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            let result = device.queue_present_khr(self.device_context.device_queue_handle.present_queue, &present_info);

            let is_changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
                || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

            if self.present_handle.window_handle.is_resized || is_changed {
                self.present_handle.window_handle.is_resized = false;
                self.recreate_swapchain()?;
            } else if let Err(e) = result {
                return Err(anyhow!(e));
            }

            self.command_handle.sync_handle.current_frame = (current_frame + 1) % max_frames_in_flight;

            //println!("Finishing took: {:?}", start.elapsed());

            Ok(())
        }
    }

    /// TEMPTEMPTEMPTEMPTEMPTEMPTEMPTEMPTEMPTEMPTEMPTEMP
    /// Updates the uniform buffer object for the Vulkan app
    fn update_uniform_buffer(&self, frame_index: usize) -> Result<()> {
        // MVP

        let view = Mat4::look_at_rh(
            vec3(2.0, 2.0, 2.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );

        let mut proj = glam::Mat4::perspective_rh(
            45.0 * DEG_TO_RAD,
            self.present_handle.swapchain_handle.extent.width as f32
                / self.present_handle.swapchain_handle.extent.height as f32,
            0.1,
            10.0,
        );

        proj.col_mut(1).y *= -1.0;

        let ubo = UniformBufferObject { view, proj };

        // Copy

        unsafe {
            memcpy(&ubo, self.model_handle.uniform_buffers[frame_index].mapped.cast_mut(), 1);
        }

        Ok(())
    }

    fn update_model_matrix_buffer_descriptors(&self) -> Result<()> {
        if self.render_pipeline_handle.descriptor_handle.descriptor_sets.is_empty() {
            return Ok(());
        }

        for i in 0..MAX_FRAMES_IN_FLIGHT as usize {
            let static_model_matrix_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.model_handle.static_model_matrix_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let static_model_matrix_buffer_info = [static_model_matrix_info];
            let static_model_matrix_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.render_pipeline_handle.descriptor_handle.descriptor_sets[i])
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&static_model_matrix_buffer_info);

            let dyn_model_matrix_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.model_handle.dyn_model_matrix_buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let dyn_model_matrix_buffer_info = [dyn_model_matrix_info];
            let dyn_model_matrix_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.render_pipeline_handle.descriptor_handle.descriptor_sets[i])
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&dyn_model_matrix_buffer_info);

            unsafe {
                self.device_context.device.update_descriptor_sets(
                    &[static_model_matrix_write, dyn_model_matrix_write],
                    &[] as &[vk::CopyDescriptorSet],
                );
            }
        }

        Ok(())
    }
}

// ______________________________________________________________________________________________________________________________________________________
// Device Context Setup
// ______________________________________________________________________________________________________________________________________________________

/// Whether the validation layers should be enabled (only enabled if debug assertions flag is active)
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

/// The name of the validation layers
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

/// The required device extensions.
const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

/// The Vulkan SDK version that started requiring the portability subset extension for macOS.
const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

// Build Functions

unsafe fn create_entry() -> Result<Entry> {
    // Dev-only logger with a sensible default; RUST_LOG still overrides this.
    #[cfg(debug_assertions)]
    {
        let mut logger = pretty_env_logger::formatted_builder();
        logger.parse_filters("info");
        logger.parse_default_env();
        logger.init();
    }

    // Creates entry
    let loader = LibloadingLoader::new(LIBRARY)?;
    Entry::new(loader).map_err(|b| anyhow!("{}", b))
}

unsafe fn create_instance(
    window: &Window,
    entry: &Entry,
) -> Result<(Instance, vk::DebugUtilsMessengerEXT)> {
    // Application Info

    let application_info = vk::ApplicationInfo::builder()
        .application_name(b"Vulkan Tutorial\0")
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(b"No Engine\0")
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 1, 0));

    // Layers

    let available_layers = entry
        .enumerate_instance_layer_properties()?
        .iter()
        .map(|l| l.layer_name)
        .collect::<HashSet<_>>();

    if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
        return Err(anyhow!("Validation layer requested but not supported"));
    }

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    // Extensions

    // Get global required extensions for Vulkan to run
    let mut extensions = vk_window::get_required_instance_extensions(window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    // Add macOS required extensions if user is on macOS
    let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        info!("Enabling extensions for macOS portability");
        extensions.push(
            vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                .name
                .as_ptr(),
        );
        extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::empty()
    };

    if VALIDATION_ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    // Create

    let mut info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .flags(flags);

    let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .user_callback(Some(debug_callback));

    if VALIDATION_ENABLED {
        info = info.push_next(&mut debug_info);
    }

    let instance = entry.create_instance(&info, None)?;

    // Messenger
    let mut messenger = vk::DebugUtilsMessengerEXT::null();

    if VALIDATION_ENABLED {
        messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
    }

    Ok((instance, messenger))
}

unsafe fn pick_physical_device(
    instance: Instance,
    surface: vk::SurfaceKHR,
) -> Result<vk::PhysicalDevice> {
    let chosen_physical_device = Some(*instance.enumerate_physical_devices()?
        .iter()
        .filter_map(|p| {
            let properties = instance.get_physical_device_properties(*p);
            if let Err(error) = check_physical_device(&instance, *p, surface) {
                warn!("Skipping physical device ('{}'): {}", properties.device_name, error);
                None
            } else {
                info!("Found available physical device ('{}')\n\tDevice type ('{:?}')\n\tPush constant size ({})\n\tMax image dimension 2d ({})",
                properties.device_name,
                properties.device_type,
                properties.limits.max_push_constants_size,
                properties.limits.max_image_dimension_2d,
            );
                Some(p)
            }
        })
        .max_by_key(|p| {
            // lower score for preferred device types
            let properties = instance.get_physical_device_properties(**p);

            match properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 10000 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
                vk::PhysicalDeviceType::INTEGRATED_GPU => 1000 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
                vk::PhysicalDeviceType::VIRTUAL_GPU => 100 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
                vk::PhysicalDeviceType::CPU => 10 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
                vk::PhysicalDeviceType::OTHER => 1 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
                _ => 0
            }

        })
        .unwrap());

    if chosen_physical_device != None {
        info!(
            "Chose physical device ('{}')",
            instance
                .get_physical_device_properties(chosen_physical_device.unwrap())
                .device_name
        );
        return Ok(chosen_physical_device.unwrap());
    }

    Err(anyhow!("Failed to find suitable physical device"))
}

unsafe fn create_logical_device(
    messenger: vk::DebugUtilsMessengerEXT,
    entry: &Entry,
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> Result<Device> {
    // Queue Create Infos

    let (graphics_index, present_index) =
        get_queue_family_indices(instance, physical_device, &surface)?;

    let mut unique_indices = HashSet::new();
    unique_indices.insert(graphics_index);
    unique_indices.insert(present_index);

    let queue_priorities = &[1.0];
    let queue_infos = unique_indices
        .iter()
        .map(|i| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*i)
                .queue_priorities(queue_priorities)
        })
        .collect::<Vec<_>>();

    // Extensions

    let mut extensions = DEVICE_EXTENSIONS
        .iter()
        .map(|n| n.as_ptr())
        .collect::<Vec<_>>();

    // Required by Vulkan SDK on macOS
    if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    // Enforce shader draw parameters for slang shaders
    extensions.push(vk::KHR_SHADER_DRAW_PARAMETERS_EXTENSION.name.as_ptr());

    // Enable descriptor indexing for bindless rendering
    extensions.push(vk::EXT_DESCRIPTOR_INDEXING_EXTENSION.name.as_ptr());

    // Features

    let features = vk::PhysicalDeviceFeatures::builder()
        .sampler_anisotropy(true)
        .sample_rate_shading(true)
        .shader_int16(true)
        .multi_draw_indirect(true);

    let mut descriptor_indexing_features = vk::PhysicalDeviceVulkan12Features::builder()
        .descriptor_indexing(true)
        .descriptor_binding_sampled_image_update_after_bind(true)
        .descriptor_binding_partially_bound(true)
        .runtime_descriptor_array(true);

    let mut storage_16bit_features = vk::PhysicalDevice16BitStorageFeatures::builder()
        .storage_buffer_16bit_access(true)
        .uniform_and_storage_buffer_16bit_access(true);

    // Create

    let info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&extensions)
        .enabled_features(&features)
        .push_next(&mut storage_16bit_features)
        .push_next(&mut descriptor_indexing_features);

    let device = instance.create_device(physical_device, &info, None)?;

    Ok(device)
}

unsafe fn get_device_graphics_present_queues(
    device: Device,
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    surface: &vk::SurfaceKHR,
) -> Result<DeviceQueueHandle> {
    let (graphics_index, present_index) =
        get_queue_family_indices(instance, physical_device, surface)?;
    let graphics_queue = device.get_device_queue(graphics_index, 0);
    let present_queue = device.get_device_queue(present_index, 0);
    Ok(DeviceQueueHandle {
        graphics_queue,
        present_queue,
        graphics_queue_family_index: graphics_index,
        present_queue_family_index: present_index,
    })
}

// Helper Functions

extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        debug!("({:?}) {}", type_, message);
    } else {
        trace!("({:?}) {}", type_, message);
    }

    vk::FALSE
}

pub fn check_physical_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> Result<()> {
    get_queue_family_indices(instance, physical_device, &surface)?;
    check_physical_device_extensions(instance, physical_device)?;

    let available_formats = unsafe {
        instance
            .get_physical_device_surface_formats_khr(physical_device, surface)
            .unwrap()
    };
    let available_present_modes = unsafe {
        instance
            .get_physical_device_surface_present_modes_khr(physical_device, surface)
            .unwrap()
    };

    if available_formats.is_empty() || available_present_modes.is_empty() {
        return Err(anyhow!(SuitabilityError("Insufficient swapchain support")));
    }

    let features = unsafe { instance.get_physical_device_features(physical_device) };
    if features.sampler_anisotropy != vk::TRUE {
        return Err(anyhow!(SuitabilityError("No sampler anisotropy")));
    }

    Ok(())
}

pub fn check_physical_device_extensions(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    unsafe {
        let extensions = instance
            .enumerate_device_extension_properties(physical_device, None)?
            .iter()
            .map(|e| e.extension_name)
            .collect::<HashSet<_>>();

        if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
            Ok(())
        } else {
            Err(anyhow!(SuitabilityError(
                "Missing required device extensions"
            )))
        }
    }
}

pub fn get_queue_family_indices(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    surface: &vk::SurfaceKHR,
) -> Result<(u32, u32)> {
    unsafe {
        let properties = instance.get_physical_device_queue_family_properties(physical_device);

        // Get graphics queue
        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        // Get present queue
        let mut present = None;
        for (index, properties) in properties.iter().enumerate() {
            if instance.get_physical_device_surface_support_khr(
                physical_device,
                index as u32,
                *surface,
            )? {
                present = Some(index as u32);
                break;
            }
        }

        if let (Some(graphics), Some(present)) = (graphics, present) {
            Ok((graphics, present))
        } else {
            Err(anyhow!(SuitabilityError("Missing required queue families")))
        }
    }
}

pub fn get_memory_type_index(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    unsafe {
        let memory = instance.get_physical_device_memory_properties(physical_device);
        (0..memory.memory_type_count)
            .find(|i| {
                let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
                let memory_type = memory.memory_types[*i as usize];
                suitable && memory_type.property_flags.contains(properties)
            })
            .ok_or_else(|| anyhow!("Failed to find suitable memory type"))
    }
}

// ______________________________________________________________________________________________________________________________________________________
// Present Engine Setup
// ______________________________________________________________________________________________________________________________________________________

// Build Functions

unsafe fn create_window(with_fullscreen: bool) -> Result<(EventLoop<()>, Window)> {
    // On Linux, winit expects WAYLAND_DISPLAY, WAYLAND_SOCKET or DISPLAY to be set.
    // If the environment doesn't provide any of these, try to detect a common
    // Wayland socket location (e.g. $XDG_RUNTIME_DIR/wayland-0 or /run/wayland-0)
    // and set `WAYLAND_DISPLAY=wayland-0` so winit can connect when appropriate.
    ensure_wayland_env();

    // Window

    let event_loop: EventLoop<()> = EventLoop::new()?;

    let window: Window = WindowBuilder::new()
        .with_title("Vulkanalia Game")
        .with_inner_size(LogicalSize::new(2560, 1440))
        .build(&event_loop)
        .unwrap();

    // Set fullscreen if enabled
    if with_fullscreen
        && let Some(monitor) = window
            .current_monitor()
            .or_else(|| window.primary_monitor())
    {
        if let Some(video_mode) = monitor
            .video_modes()
            //.max_by_key(|mode| mode.refresh_rate_millihertz() + mode.size().width * mode.size().height)
            .find(|mode| {
                mode.refresh_rate_millihertz() / 1000 == 240
                    && mode.size().width == 2560
                    && mode.size().height == 1440
            })
        {
            window.set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())));

            println!(
                "\nDisplay: {}x{}@{}Hz\n",
                video_mode.size().width,
                video_mode.size().height,
                video_mode.refresh_rate_millihertz() / 1000
            );
        }
    }

    Ok((event_loop, window))
}

unsafe fn create_surface(instance: Instance, window: &Window) -> Result<vk::SurfaceKHR> {
    // Surface
    Ok(vulkanalia::window::create_surface(
        &instance, window, window,
    )?)
}

fn set_default_msaa(device_context: &DeviceContext) -> vulkanalia::vk::SampleCountFlags {
    let max_msaa = get_max_msaa_samples(device_context.clone().instance, device_context.physical_device);
    let chosen_msaa = if max_msaa < vk::SampleCountFlags::_4 {
        max_msaa
    } else {
        vk::SampleCountFlags::_4
    };
    info!("Max msaa detected: {:?}", max_msaa);
    info!("Chosen msaa: {:?}", chosen_msaa);
    chosen_msaa
}

unsafe fn create_swapchain(
    device_context: &DeviceContext,
    window: &Window,
    surface: vk::SurfaceKHR,
) -> Result<SwapchainHandle> {
    // Image

    let swapchain_capabilities =
        device_context.instance.get_physical_device_surface_capabilities_khr(device_context.physical_device, surface)?;

    let surface_format = get_swapchain_surface_format(device_context.clone().instance, device_context.physical_device, surface);
    let present_mode = get_swapchain_present_mode(device_context.clone().instance, device_context.physical_device, surface);
    let swapchain_extent = get_swapchain_extent(window, swapchain_capabilities);
    let swapchain_format = surface_format.format;

    let mut image_count = swapchain_capabilities.min_image_count + 1;
    if swapchain_capabilities.max_image_count != 0
        && image_count > swapchain_capabilities.max_image_count
    {
        image_count = swapchain_capabilities.max_image_count
    }

    let mut queue_family_indices = vec![];
    let image_sharing_mode = if device_context.device_queue_handle.graphics_queue_family_index
        != device_context.device_queue_handle.present_queue_family_index
    {
        queue_family_indices.push(device_context.device_queue_handle.graphics_queue_family_index);
        queue_family_indices.push(device_context.device_queue_handle.present_queue_family_index);
        vk::SharingMode::CONCURRENT
    } else {
        vk::SharingMode::EXCLUSIVE
    };

    // Create

    let info = vk::SwapchainCreateInfoKHR::builder()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(swapchain_extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(image_sharing_mode)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(swapchain_capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null());

    let swapchain = device_context.device.create_swapchain_khr(&info, None)?;

    // Images

    let swapchain_images = device_context.device.get_swapchain_images_khr(swapchain)?;

    // Image Views

    let swapchain_image_views = swapchain_images
        .iter()
        .map(|i| {
            create_image_view(
                device_context.device.clone(),
                *i,
                swapchain_format,
                vk::ImageAspectFlags::COLOR,
                1,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SwapchainHandle {
        swapchain,
        images: swapchain_images,
        image_views: swapchain_image_views,
        format: swapchain_format,
        extent: swapchain_extent,
    })
}

unsafe fn create_color_texture(
    device_context: &DeviceContext,
    swapchain_handle: &SwapchainHandle,
    msaa_samples: vk::SampleCountFlags,
) -> Result<Texture> {
    // Image + Image Memory

    let (color_image, color_image_memory) = create_image(
        device_context.clone(),
        swapchain_handle.extent.width,
        swapchain_handle.extent.height,
        1,
        msaa_samples,
        swapchain_handle.format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let color_image = color_image;
    let color_image_memory = color_image_memory;

    // Image View

    let color_image_view = create_image_view(
        device_context.clone().device,
        color_image,
        swapchain_handle.format,
        vk::ImageAspectFlags::COLOR,
        1,
    )?;

    Ok(Texture {
        image: color_image,
        image_memory: color_image_memory,
        image_view: color_image_view,
    })
}

unsafe fn create_depth_texture(
    device_context: &DeviceContext,
    swapchain_handle: &SwapchainHandle,
    msaa_samples: vk::SampleCountFlags,
) -> Result<Texture> {
    // Image + Image Memory

    let format = get_depth_format(device_context.clone().instance, device_context.physical_device)?;

    let (depth_image, depth_image_memory) = create_image(
        device_context.clone(),
        swapchain_handle.extent.width,
        swapchain_handle.extent.height,
        1,
        msaa_samples,
        format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    )?;

    let depth_image = depth_image;
    let depth_image_memory = depth_image_memory;

    // Image view

    let depth_image_view = create_image_view(
        device_context.clone().device,
        depth_image,
        format,
        vk::ImageAspectFlags::DEPTH,
        1,
    )?;

    Ok(Texture {
        image: depth_image,
        image_memory: depth_image_memory,
        image_view: depth_image_view,
    })
}

// Helper Functions

pub fn get_max_msaa_samples(
    instance: Instance,
    physical_device: vk::PhysicalDevice,
) -> vk::SampleCountFlags {
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let counts = properties.limits.framebuffer_color_sample_counts
        & properties.limits.framebuffer_depth_sample_counts;
    [
        vk::SampleCountFlags::_64,
        vk::SampleCountFlags::_32,
        vk::SampleCountFlags::_16,
        vk::SampleCountFlags::_8,
        vk::SampleCountFlags::_4,
        vk::SampleCountFlags::_2,
    ]
    .iter()
    .cloned()
    .find(|c| counts.contains(*c))
    .unwrap_or(vk::SampleCountFlags::_1)
}

pub unsafe fn get_swapchain_surface_format(
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> vk::SurfaceFormatKHR {
    let formats = instance
        .get_physical_device_surface_formats_khr(physical_device, surface)
        .unwrap();
    let format = formats
        .iter()
        .cloned()
        .find(|f| {
            (f.format == vk::Format::B8G8R8_SRGB || f.format == vk::Format::R8G8B8_SRGB)
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .or_else(|| {
            formats
                .iter()
                .cloned()
                .find(|f| f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
        })
        .unwrap_or_else(|| formats[0]);

    info!(
        "Selected swapchain format: {:?}, color space: {:?}",
        format.format, format.color_space
    );
    format
}

#[rustfmt::skip]
pub fn get_swapchain_extent(window: &Window, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D::builder()
            .width(window.inner_size().width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ))
            .height(window.inner_size().height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ))
            .build()
    }
}

pub unsafe fn get_swapchain_present_mode(
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> vk::PresentModeKHR {
    let present_modes = instance
        .get_physical_device_surface_present_modes_khr(physical_device, surface)
        .unwrap();

    present_modes
        .iter()
        .cloned()
        .find(|m| *m == vk::PresentModeKHR::IMMEDIATE)
        .or_else(|| {
            present_modes
                .iter()
                .cloned()
                .find(|m| *m == vk::PresentModeKHR::MAILBOX)
        })
        .unwrap_or(vk::PresentModeKHR::FIFO)
}

pub fn get_depth_format(instance: Instance, physical_device: vk::PhysicalDevice) -> Result<vk::Format> {
    let candidates = &[
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    get_supported_image_format(
        instance,
        physical_device,
        candidates,
        vk::ImageTiling::OPTIMAL,
        vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
    )
}

#[cfg(target_os = "linux")]
unsafe fn ensure_wayland_env() {
    use std::env;
    use std::path::Path;

    if env::var_os("WAYLAND_DISPLAY").is_none()
        && env::var_os("WAYLAND_SOCKET").is_none()
        && env::var_os("DISPLAY").is_none()
    {
        if let Some(xdg) = env::var_os("XDG_RUNTIME_DIR") {
            let p = Path::new(&xdg).join("wayland-0");
            if p.exists() {
                env::set_var("WAYLAND_DISPLAY", "wayland-0");
                return;
            }
        }

        let candidates = ["/run/user/1000/wayland-0", "/run/wayland-0"];
        for c in candidates {
            if Path::new(c).exists() {
                env::set_var("WAYLAND_DISPLAY", "wayland-0");
                return;
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
unsafe fn ensure_wayland_env() {}

// ______________________________________________________________________________________________________________________________________________________
// Render Pipeline Engine Setup
// ______________________________________________________________________________________________________________________________________________________

/// Maximum number of textures that can be loaded in memory at any one time
const BINDLESS_TEXTURE_COUNT: u32 = 5_000;

// Build Functions

unsafe fn create_base_render_pass(
    device_context: &DeviceContext,
    swapchain_handle: &SwapchainHandle,
    msaa_samples: vk::SampleCountFlags,
) -> Result<vk::RenderPass> {
    // Attachments

    let color_attachment = vk::AttachmentDescription::builder()
        .format(swapchain_handle.format)
        .samples(msaa_samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_stencil_attachment = vk::AttachmentDescription::builder()
        .format(get_depth_format(device_context.clone().instance, device_context.physical_device)?)
        .samples(msaa_samples)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_resolve_attachment = vk::AttachmentDescription::builder()
        .format(swapchain_handle.format)
        .samples(vk::SampleCountFlags::_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    // Subpasses

    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_stencil_attachment_ref = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_resolve_attachment_ref = vk::AttachmentReference::builder()
        .attachment(2)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = &[color_attachment_ref];
    let resolve_attachments = &[color_resolve_attachment_ref];
    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(color_attachments)
        .depth_stencil_attachment(&depth_stencil_attachment_ref)
        .resolve_attachments(resolve_attachments);

    // Dependencies

    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );

    // Create

    let attachments = &[
        color_attachment,
        depth_stencil_attachment,
        color_resolve_attachment,
    ];
    let subpasses = &[subpass];
    let dependencies = &[dependency];
    let info = vk::RenderPassCreateInfo::builder()
        .attachments(attachments)
        .subpasses(subpasses)
        .dependencies(dependencies);

    let render_pass = device_context.device.create_render_pass(&info, None)?;

    Ok(render_pass)
}

unsafe fn create_descriptor_set_layout(device: Device) -> Result<vk::DescriptorSetLayout> {
    let texture_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
        .descriptor_count(BINDLESS_TEXTURE_COUNT)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_type(vk::DescriptorType::SAMPLER)
        .descriptor_count(BINDLESS_TEXTURE_COUNT)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(2)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let static_model_matrix_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(3)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let dyn_model_matrix_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(4)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let indirect_draw_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(5)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let instance_data_binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(6)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    let binding_flags = &[
        vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
    ];
    let mut layout_flags =
        vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder().binding_flags(binding_flags);

    let bindings = &[
        texture_binding,
        sampler_binding,
        ubo_binding,
        static_model_matrix_binding,
        dyn_model_matrix_binding,
        indirect_draw_binding,
        instance_data_binding,
    ];
    let info = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(bindings)
        .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
        .push_next(&mut layout_flags);

    let descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

    Ok(descriptor_set_layout)
}

unsafe fn create_descriptor_pool(device: Device) -> Result<vk::DescriptorPool> {
    let texture_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::SAMPLED_IMAGE)
        .descriptor_count(BINDLESS_TEXTURE_COUNT * MAX_FRAMES_IN_FLIGHT);

    let sampler_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::SAMPLER)
        .descriptor_count(BINDLESS_TEXTURE_COUNT * MAX_FRAMES_IN_FLIGHT);

    let ubo_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT);

    let static_model_matrix_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT);

    let dyn_model_matrix_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT);

    let indirect_draw_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT);

    let instance_data_size = vk::DescriptorPoolSize::builder()
        .type_(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT);

    let pool_sizes = &[
        texture_size,
        sampler_size,
        ubo_size,
        static_model_matrix_size,
        dyn_model_matrix_size,
        indirect_draw_size,
        instance_data_size,
    ];
    let info = vk::DescriptorPoolCreateInfo::builder()
        .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
        .pool_sizes(pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT);

    let descriptor_pool = device.create_descriptor_pool(&info, None)?;

    Ok(descriptor_pool)
}

unsafe fn create_descriptor_sets(
    device_context: &DeviceContext,
    model_handle: &ModelHandle,
    main_camera_visbuffers: &Vec<Visbuffer>,
    texture_handle: &TextureHandle,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
) -> Result<Vec<vk::DescriptorSet>> {
    // Allocate

    let layouts = vec![descriptor_set_layout; MAX_FRAMES_IN_FLIGHT as usize];
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&layouts);

    let descriptor_sets = device_context.device.allocate_descriptor_sets(&info)?;

    // Update

    for i in 0..MAX_FRAMES_IN_FLIGHT as usize {
        let ubo_info = [vk::DescriptorBufferInfo::builder()
            .buffer(model_handle.uniform_buffers[i].buffer)
            .offset(0)
            .range(size_of::<UniformBufferObject>() as u64)];

        let ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[i])
            .dst_binding(2)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&ubo_info);

        let static_model_matrix_info = [vk::DescriptorBufferInfo::builder()
            .buffer(model_handle.static_model_matrix_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)];

        let static_model_matrix_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[i])
            .dst_binding(3)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&static_model_matrix_info);

        let dyn_model_matrix_info = [vk::DescriptorBufferInfo::builder()
            .buffer(model_handle.dyn_model_matrix_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)];

        let dyn_model_matrix_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[i])
            .dst_binding(4)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&dyn_model_matrix_info);

        let indirect_draw_info = [vk::DescriptorBufferInfo::builder()
            .buffer(main_camera_visbuffers[i].indirect_draw_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)];

        let indirect_draw_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[i])
            .dst_binding(5)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&indirect_draw_info);

        let instance_data_info = [vk::DescriptorBufferInfo::builder()
            .buffer(main_camera_visbuffers[i].instance_buffer.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE)];

        let instance_data_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[i])
            .dst_binding(6)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&instance_data_info);

        device_context.device.update_descriptor_sets(
            &[
                ubo_write,
                static_model_matrix_write,
                dyn_model_matrix_write,
                indirect_draw_write,
                instance_data_write,
            ],
            &[] as &[vk::CopyDescriptorSet],
        );
    }

    refresh_bindless_textures(device_context, &descriptor_sets, texture_handle)?;

    Ok(descriptor_sets)
}

unsafe fn create_pipeline(
    swapchain_handle: &SwapchainHandle,
    msaa_samples: vk::SampleCountFlags,
    descriptor_set_layout: vk::DescriptorSetLayout,
    render_pass: vk::RenderPass,
    device_context: &DeviceContext,
) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    // Stages

    let shader = include_bytes!("../../assets/shaders/shader.spv");

    let shader_module = create_shader_module(&device_context.device, &shader[..])?;

    let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(shader_module)
        .name(b"vertMain\0");

    let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(shader_module)
        .name(b"fragMain\0");

    // Vertex Input State

    let binding_descriptions = &[QuantizedVertex::binding_description()];
    let attribute_descriptions =
        QuantizedVertex::attribute_descriptions(&device_context.instance, &device_context.physical_device)?;
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_binding_descriptions(binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    // Input Assembly State

    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // Viewport State

    let viewport = vk::Viewport::builder()
        .x(0.0)
        .y(0.0)
        .width(swapchain_handle.extent.width as f32)
        .height(swapchain_handle.extent.height as f32)
        .min_depth(0.0)
        .max_depth(1.0);

    let scissor = vk::Rect2D::builder()
        .offset(vk::Offset2D { x: 0, y: 0 })
        .extent(swapchain_handle.extent);

    let viewports = &[viewport];
    let scissors = &[scissor];
    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(viewports)
        .scissors(scissors);

    // Rasterization State

    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    // Multisample State

    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(true)
        .min_sample_shading(0.5)
        .rasterization_samples(msaa_samples);

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false);

    // Color Blend State

    let attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(vk::ColorComponentFlags::all())
        .blend_enable(false);

    let attachments = &[attachment];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(attachments)
        .blend_constants([0.0, 0.0, 0.0, 0.0]);

    // Layout

    let set_layouts = &[descriptor_set_layout];
    let push_constant_ranges = &[vk::PushConstantRange::builder()
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(size_of::<PushConstant>() as u32)
        .build()];
    let layout_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(set_layouts)
        .push_constant_ranges(push_constant_ranges);
    let pipeline_layout = device_context.device.create_pipeline_layout(&layout_info, None)?;

    // Create

    let stages = &[vert_stage, frag_stage];
    let info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0);

    let pipeline = device_context.device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
        .0[0];

    // Cleanup

    device_context.device.destroy_shader_module(shader_module, None);

    Ok((pipeline, pipeline_layout))
}

unsafe fn create_framebuffers(
    swapchain_handle: &SwapchainHandle,
    color_texture: Texture,
    depth_texture: Texture,
    render_pass: vk::RenderPass,
    device: Device,
) -> Result<Vec<vk::Framebuffer>> {
    let framebuffers = swapchain_handle
        .image_views
        .iter()
        .map(|i| {
            let attachments = &[color_texture.image_view, depth_texture.image_view, *i];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(attachments)
                .width(swapchain_handle.extent.width)
                .height(swapchain_handle.extent.height)
                .layers(1);

            device.create_framebuffer(&create_info, None)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(framebuffers)
}

// Helper Functions

pub fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    unsafe {
        let bytecode = Bytecode::new(bytecode).unwrap();
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(bytecode.code())
            .code_size(bytecode.code_size());
        Ok(device.create_shader_module(&info, None)?)
    }
}

// ______________________________________________________________________________________________________________________________________________________
// Command Engine Setup
// ______________________________________________________________________________________________________________________________________________________

const DEG_TO_RAD: f32 = PI / 180.0;

// Build Functions

unsafe fn create_command_pool(
    device: Device,
    device_queue_handle: DeviceQueueHandle,
) -> Result<vk::CommandPool> {
    let info = vk::CommandPoolCreateInfo::builder()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(device_queue_handle.graphics_queue_family_index);

    let command_pool = device.create_command_pool(&info, None)?;

    Ok(command_pool)
}

unsafe fn create_command_buffers(
    device: Device,
    command_pool: vk::CommandPool,
    framebuffer_count: usize,
) -> Result<Vec<vk::CommandBuffer>> {
    // Allocate

    let allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(framebuffer_count as u32);

    let command_buffers = device.allocate_command_buffers(&allocate_info)?;

    Ok(command_buffers)
}

unsafe fn create_sync_objects(
    device: Device,
    swapchain_handle: &SwapchainHandle,
) -> Result<SyncHandle> {
    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

    let mut image_available_semaphores: Vec<vk::Semaphore> = Vec::new();
    let mut in_flight_fences: Vec<vk::Fence> = Vec::new();
    let mut render_finished_semaphores: Vec<vk::Semaphore> = Vec::new();

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        image_available_semaphores.push(device.create_semaphore(&semaphore_info, None)?);
        in_flight_fences.push(device.create_fence(&fence_info, None)?);
    }

    for _ in 0..swapchain_handle.images.len() {
        render_finished_semaphores.push(device.create_semaphore(&semaphore_info, None)?);
    }

    let images_in_flight: Vec<vk::Fence> = swapchain_handle
        .images
        .iter()
        .map(|_| vk::Fence::null())
        .collect();

    Ok(SyncHandle {
        image_available_semaphores,
        render_finished_semaphores,
        in_flight_fences,
        images_in_flight,
        max_frames_in_flight: MAX_FRAMES_IN_FLIGHT as usize,
        current_frame: 0,
    })
}

fn create_source_instance_buffer(device_context: &DeviceContext, command_pool: vk::CommandPool) -> Buffer<PerInstanceData> {
    Buffer::new(
        device_context,
        command_pool,
        4096,
        vk::BufferUsageFlags::STORAGE_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        4096,
        Vec::new(),
        true,
    )
}

fn create_visbuffers(
    device_context: &DeviceContext,
    command_pool: vk::CommandPool
) -> Vec<Visbuffer> {
    let mut visbuffers: Vec<Visbuffer> = Vec::new();
    for i in 0..MAX_FRAMES_IN_FLIGHT {
        visbuffers.push(Visbuffer::new(device_context, command_pool, 1));
    }
    visbuffers
}

// Helper Functions

pub fn begin_single_time_commands(
    command_pool: vk::CommandPool,
    device: Device,
) -> Result<vk::CommandBuffer> {
    unsafe {
        // Allocate

        let info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);

        let command_buffer = device.allocate_command_buffers(&info)?[0];

        // Begin

        let info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device.begin_command_buffer(command_buffer, &info)?;

        Ok(command_buffer)
    }
}

pub fn end_single_time_commands(
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    device: Device,
    device_queue_handle: DeviceQueueHandle,
) -> Result<()> {
    unsafe {
        // End

        device.end_command_buffer(command_buffer)?;

        // Submit

        let command_buffers = &[command_buffer];
        let info = vk::SubmitInfo::builder().command_buffers(command_buffers);

        device.queue_submit(
            device_queue_handle.graphics_queue,
            &[info],
            vk::Fence::null(),
        )?;
        device.queue_wait_idle(device_queue_handle.graphics_queue)?;

        // Cleanup

        device.free_command_buffers(command_pool, &[command_buffer]);

        Ok(())
    }
}

// ______________________________________________________________________________________________________________________________________________________
// Model Engine Setup
// ______________________________________________________________________________________________________________________________________________________

/// Allocates/Deallocates 32KiB at a time
const MODEL_MATRIX_ALLOCATE_THRESHOLD: u32 = 1024;

// Build Functions

fn create_vertex_index_buffers(
    device_context: DeviceContext,
    command_pool: vk::CommandPool,
) -> (Buffer<QuantizedVertex>, Buffer<u32>) {
    let vertex_buffer: Buffer<QuantizedVertex> = Buffer::new(
        &device_context,
        command_pool,
        1,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        0,
        Vec::new(),
        false,
    );

    let index_buffer: Buffer<u32> = Buffer::new(
        &device_context,
        command_pool,
        1,
        vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        0,
        Vec::new(),
        false,
    );

    (vertex_buffer, index_buffer)
}

fn create_uniform_buffers(
    device_context: &DeviceContext,
    command_pool: vk::CommandPool,
) -> Vec<Buffer<UniformBufferObject>> {
    let mut uniform_buffers: Vec<Buffer<UniformBufferObject>> = Vec::new();

    for _ in 0..MAX_FRAMES_IN_FLIGHT {
        let buffer: Buffer<UniformBufferObject> = Buffer::new(
            device_context,
            command_pool,
            262_144,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            0,
            Vec::new(),
            false,
        );
        uniform_buffers.push(buffer);
    }

    uniform_buffers
}

fn create_model_matrix_buffers(
    device_context: DeviceContext,
    command_pool: vk::CommandPool,
) -> (Buffer<QuantizedModelMatrix>, Buffer<QuantizedModelMatrix>) {
    let dyn_model_matrix_buffer: Buffer<QuantizedModelMatrix> = Buffer::new(
        &device_context,
        command_pool,
        MODEL_MATRIX_ALLOCATE_THRESHOLD as u64,
        vk::BufferUsageFlags::STORAGE_BUFFER,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        MODEL_MATRIX_ALLOCATE_THRESHOLD,
        Vec::new(),
        true,
    );
    let static_model_matrix_buffer: Buffer<QuantizedModelMatrix> = Buffer::new(
        &device_context,
        command_pool,
        MODEL_MATRIX_ALLOCATE_THRESHOLD as u64,
        vk::BufferUsageFlags::STORAGE_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        MODEL_MATRIX_ALLOCATE_THRESHOLD,
        Vec::new(),
        true,
    );
    (dyn_model_matrix_buffer, static_model_matrix_buffer)
}

fn create_mesh_buffer(
    device_context: DeviceContext,
    command_pool: vk::CommandPool,
) -> Buffer<MeshBufferLayout> {
    Buffer::new(
        &device_context,
        command_pool,
        1,
        vk::BufferUsageFlags::UNIFORM_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        0,
        vec![MeshBufferLayout::default()],
    false,
    )
}

// Helper Functions

pub fn get_supported_vertex_format(
    device_context: &DeviceContext,
    candidates: &[vk::Format],
    features: vk::FormatFeatureFlags,
) -> Result<vk::Format> {
    candidates
        .iter()
        .cloned()
        .find(|f| {
            let properties = unsafe {
                device_context
                    .instance
                    .get_physical_device_format_properties(device_context.physical_device, *f)
            };
            // For vertex buffers, check buffer features (typically linear tiling)
            properties.buffer_features.contains(features)
        })
        .ok_or_else(|| anyhow!("Failed to find supported vertex attribute format"))
}

// ______________________________________________________________________________________________________________________________________________________
// Texture Engine Setup
// ______________________________________________________________________________________________________________________________________________________

// Helper Functions

pub fn get_supported_image_format(
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    candidates: &[vk::Format],
    tiling: vk::ImageTiling,
    features: vk::FormatFeatureFlags,
) -> Result<vk::Format> {
    unsafe {
        candidates
            .iter()
            .cloned()
            .find(|f| {
                let properties =
                    instance.get_physical_device_format_properties(physical_device, *f);
                match tiling {
                    vk::ImageTiling::LINEAR => {
                        properties.linear_tiling_features.contains(features)
                    }
                    vk::ImageTiling::OPTIMAL => {
                        properties.optimal_tiling_features.contains(features)
                    }
                    _ => false,
                }
            })
            .ok_or_else(|| anyhow!("Failed to find supported format"))
    }
}

pub fn is_image_format_supported(
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    format: vk::Format,
) -> bool {
    let properties =
        unsafe { instance.get_physical_device_format_properties(physical_device, format) };
    // Check if format is supported for optimal tiling with sampled image feature
    properties
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE)
}

pub fn create_sampler(device: Device, sampler_contents: SamplerContents) -> vk::Sampler {
    // Create sampler
    let info = vk::SamplerCreateInfo::builder()
        .mag_filter(sampler_contents.filter)
        .min_filter(sampler_contents.filter)
        .address_mode_u(sampler_contents.address_mode_u)
        .address_mode_v(sampler_contents.address_mode_v)
        .address_mode_w(sampler_contents.address_mode_w)
        .anisotropy_enable(true)
        .max_anisotropy(16.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(sampler_contents.mipmap_mode)
        .min_lod(0.0)
        .max_lod(sampler_contents.mipmap_levels as f32)
        .mip_lod_bias(0.0);
    unsafe { device.create_sampler(&info, None).unwrap() }
}

pub fn create_image(
    device_context: DeviceContext,
    width: u32,
    height: u32,
    mipmap_levels: u32,
    samples: vk::SampleCountFlags,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    // Image

    let info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(mipmap_levels)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(samples);

    unsafe {
        let image = device_context.device.create_image(&info, None)?;

        // Memory

        let requirements = device_context.device.get_image_memory_requirements(image);

        let info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(get_memory_type_index(&device_context.instance, device_context.physical_device, properties, requirements)?);

        let image_memory = device_context.device.allocate_memory(&info, None)?;

        device_context.device.bind_image_memory(image, image_memory, 0)?;

        Ok((image, image_memory))
    }
}

pub fn create_image_view(
    device: Device,
    image: vk::Image,
    format: vk::Format,
    aspects: vk::ImageAspectFlags,
    mipmap_levels: u32,
) -> Result<vk::ImageView> {
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(aspects)
        .base_mip_level(0)
        .level_count(mipmap_levels)
        .base_array_layer(0)
        .layer_count(1);

    let info = vk::ImageViewCreateInfo::builder()
        .image(image)
        .view_type(vk::ImageViewType::_2D)
        .format(format)
        .subresource_range(subresource_range);

    unsafe { Ok(device.create_image_view(&info, None)?) }
}

pub fn transition_image_layout(
    device_context: DeviceContext,
    command_pool: vk::CommandPool,
    image: vk::Image,
    format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    mipmap_levels: u32,
) -> Result<()> {
    let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) =
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            (
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            _ => return Err(anyhow!("Unsupported image layout transition!")),
        };

    let command_buffer = begin_single_time_commands(command_pool, device_context.device.clone())?;

    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(mipmap_levels)
        .base_array_layer(0)
        .layer_count(1);

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);

    unsafe {
        device_context.device.cmd_pipeline_barrier(
            command_buffer,
            src_stage_mask,
            dst_stage_mask,
            vk::DependencyFlags::empty(),
            &[] as &[vk::MemoryBarrier],
            &[] as &[vk::BufferMemoryBarrier],
            &[barrier],
        );
    }

    end_single_time_commands(command_pool, command_buffer, device_context.device, device_context.device_queue_handle)?;

    Ok(())
}

pub fn update_bindless_texture(
    device_context: &DeviceContext,
    descriptor_sets: &Vec<vk::DescriptorSet>,
    texture_handle: &TextureHandle,
    slot_index: u32,
    view: vk::ImageView,
) -> Result<()> {
    if descriptor_sets.is_empty() {
        return Ok(());
    }

    let info = vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)

        .image_view(view);

    let image_info = &[info];
    for descriptor_set in descriptor_sets {
        let write_set = vk::WriteDescriptorSet::builder()
            .dst_set(*descriptor_set)
            .dst_binding(0)
            .dst_array_element(slot_index)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .image_info(image_info);

        unsafe {
            device_context.device.update_descriptor_sets(&[write_set], &[] as &[vk::CopyDescriptorSet]);
        }
    }
    Ok(())
}

pub fn update_bindless_sampler(
    device_context: &DeviceContext,
    descriptor_sets: &Vec<vk::DescriptorSet>,
    texture_handle: &TextureHandle,
    slot_index: u32,
    sampler: vk::Sampler,
) -> Result<()> {
    if descriptor_sets.is_empty() {
        return Ok(());
    }

    let info = vk::DescriptorImageInfo::builder().sampler(sampler);
    let sampler_info = &[info];
    for descriptor_set in descriptor_sets {
        let write_set = vk::WriteDescriptorSet::builder()
            .dst_set(*descriptor_set)
            .dst_binding(1)
            .dst_array_element(slot_index)
            .descriptor_type(vk::DescriptorType::SAMPLER)
            .image_info(sampler_info);

        unsafe {
            device_context.device.update_descriptor_sets(&[write_set], &[] as &[vk::CopyDescriptorSet]);
        }
    }
    Ok(())
}

pub fn refresh_bindless_textures(
    device_context: &DeviceContext,
    descriptor_sets: &Vec<vk::DescriptorSet>,
    texture_handle: &TextureHandle
) -> Result<()> {
    for texture in texture_handle.loaded_textures.values() {
        update_bindless_texture(
            device_context,
            descriptor_sets,
            texture_handle,
            texture.slot_index,
            texture.texture.image_view,
        )?;
    }
    for sampler_usage in texture_handle.samplers.values() {
        update_bindless_sampler(
            device_context,
            descriptor_sets,
            texture_handle,
            sampler_usage.slot_index,
            sampler_usage.sampler,
        )?;
    }

    Ok(())
}