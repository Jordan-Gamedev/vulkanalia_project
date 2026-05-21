#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::{anyhow, Result};
use log::*;
use std::collections::HashMap;
use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::{self};
use crate::engine::{CommandEngine, ModelEngine, RenderPipelineEngine};
use super::device_context::DeviceContext;

#[derive(Clone, Default)]
pub struct TextureEngine {
    loaded_textures: HashMap<String, Texture>,
    available_slots: Vec<u32>,
}

impl TextureEngine {
    pub fn destroy(&mut self, device: Device) {
        unsafe {
            self.loaded_textures.values().for_each(|t| {
                device.destroy_sampler(t.sampler, None);
                device.destroy_image_view(t.image_view, None);
                device.destroy_image(t.image, None);
                device.free_memory(t.memory, None);
            });
        }
        self.loaded_textures.clear();
        self.available_slots.clear();
    }

    pub fn load_texture(&mut self, context: DeviceContext, rp_engine: RenderPipelineEngine, command_engine: CommandEngine, path: String) -> Result<()> {
        if let Some(texture) = self.loaded_textures.get_mut(&path) {
            texture.instance_count += 1;
            return Ok(());
        }

        // Load
    
        let texture = {
            let image = include_bytes!("../../assets/textures/cuttlefish_albedo.ktx2");
            let mut texture = ktx2_rw::Ktx2Texture::from_memory(image)?;
            let context = context.clone();

            // Try BC7 first, fall back to ASTC 4x4 if not supported
            let transcode_format = if TextureEngine::is_texture_format_supported(context.clone().instance, context.physical_device, vk::Format::BC7_SRGB_BLOCK) {
                info!("Using BC7 format for texture transcoding");
                ktx2_rw::TranscodeFormat::Bc7Rgba
            } else if TextureEngine::is_texture_format_supported(context.instance, context.physical_device, vk::Format::ASTC_4X4_SRGB_BLOCK) {
                info!("BC7 not supported, falling back to ASTC 4x4 for texture transcoding");
                ktx2_rw::TranscodeFormat::Astc_4x4_Rgba
            } else {
                return Err(anyhow!("Neither BC7 nor ASTC 4x4 compression formats are supported"));
            };
            
            texture.transcode_basis(transcode_format).expect("Failed to transcode texture image format");
            texture
        };
    
        let format = vk::Format::from_raw(texture.vk_format().as_raw() as i32);
        let pixel_data = texture.get_image_data(0, 0, 0).unwrap();
        let mipmap_levels = texture.levels();
    
        // Calculate total size for all mip levels and collect per-level data
        let mut mip_sizes: Vec<usize> = Vec::with_capacity(mipmap_levels as usize);
        let mut total_size: u64 = 0;
        for level in 0..mipmap_levels {
            let mip_pixel_data = texture.get_image_data(level, 0, 0).unwrap();
            mip_sizes.push(mip_pixel_data.len());
            total_size += mip_pixel_data.len() as u64;
        }
    
        // Create (staging)
    
        let (staging_buffer, staging_buffer_memory) = ModelEngine::create_buffer(
            context.clone(),
            total_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        )?;
    
        // Copy (staging)
    
        unsafe {
            let memory = context.clone().device.map_memory(staging_buffer_memory, 0, total_size, vk::MemoryMapFlags::empty())?;
        
            // Copy each mip level into the staging buffer at the correct offset
            let mut offset: usize = 0;
            for level in 0..mipmap_levels as usize {
                let mip_pixel_data = texture.get_image_data(level as u32, 0, 0).unwrap();
                memcpy(mip_pixel_data.as_ptr(), memory.add(offset).cast(), mip_pixel_data.len());
                offset += mip_pixel_data.len();
            }
        
            context.clone().device.unmap_memory(staging_buffer_memory);    
        }

        // Create (Image)
    
        let (texture_image, texture_image_memory) = TextureEngine::create_image(
            context.clone(),
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
    
        TextureEngine::transition_image_layout(
            context.clone(),
            command_engine.clone(),
            texture_image,
            format,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            mipmap_levels,
        )?;
        
        // Copy each mip level from the staging buffer into the corresponding image mip level
        let command_buffer = command_engine.begin_single_time_commands(context.clone().device)?;
    
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
                .image_extent(vk::Extent3D { width: mip_width, height: mip_height, depth: 1 })
                .build();
    
            regions.push(region);
    
            buffer_offset += mip_sizes[level as usize] as u64;
        }
    
        unsafe {
            context.device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer,
                texture_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            );
        }
    
        command_engine.end_single_time_commands(context.clone(), command_buffer)?;
    
        TextureEngine::transition_image_layout(
            context.clone(),
            command_engine,
            texture_image,
            format,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            mipmap_levels,
        )?;
    
        // Cleanup
    
        unsafe {
            context.device.destroy_buffer(staging_buffer, None);
            context.device.free_memory(staging_buffer_memory, None);
        }
    
        // Create view
        let image_view = TextureEngine::create_image_view(context.clone().device, texture_image, format, vk::ImageAspectFlags::COLOR, mipmap_levels)?;

        // Create sampler
        let info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)
            .max_anisotropy(16.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(mipmap_levels as f32)
            .mip_lod_bias(0.0);
        let image_sampler = unsafe { context.clone().device.create_sampler(&info, None)? };
        
        // Add texture to array of textures
        let slot_index: Option<u32> = if self.available_slots.len() > 0 { self.available_slots.pop() } else { Some(self.loaded_textures.len() as u32) };
        let slot_index: u32 = slot_index.unwrap();
        self.loaded_textures.insert(path.clone(), Texture { image: texture_image, memory: texture_image_memory, image_view, sampler: image_sampler, slot_index, instance_count: 1 });
        
        // Update bindless descriptor
        TextureEngine::update_bindless_texture(context.device, &rp_engine, slot_index, image_view, image_sampler)?;

        Ok(())
    }

    pub fn unload_texture(&mut self, context: DeviceContext, rp_engine: RenderPipelineEngine, path: String) -> Result<()> {
        let (unloading_texture, fully_unloaded) = if let Some(texture) = self.loaded_textures.get_mut(&path) {
            texture.instance_count -= 1;
            (*texture, texture.instance_count == 0)
        } else {
            return Err(anyhow!("Texture not found"));
        };

        if fully_unloaded {
            self.loaded_textures.remove(&path);
            self.available_slots.push(unloading_texture.slot_index);
            unsafe {
                context.device.destroy_sampler(unloading_texture.sampler, None);
                context.device.destroy_image_view(unloading_texture.image_view, None);
                context.device.destroy_image(unloading_texture.image, None);
                context.device.free_memory(unloading_texture.memory, None);
            }
        }

        Ok(())
    }

    pub fn refresh_bindless_textures(&self, device: Device, rp_engine: &RenderPipelineEngine) -> Result<()> {
        for texture in self.loaded_textures.values() {
            TextureEngine::update_bindless_texture(device.clone(), rp_engine, texture.slot_index, texture.image_view, texture.sampler)?;
        }

        Ok(())
    }

    pub fn create_image(
        context: DeviceContext,
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
                depth: 1
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
            let image = context.device.create_image(&info, None)?;

            // Memory

            let requirements = context.device.get_image_memory_requirements(image);

            let info = vk::MemoryAllocateInfo::builder()
                .allocation_size(requirements.size)
                .memory_type_index(context.get_memory_type_index(properties, requirements)?);

            let image_memory = context.device.allocate_memory(&info, None)?;

            context.device.bind_image_memory(image, image_memory, 0)?;

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

    pub fn get_supported_format(
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
                let properties = instance.get_physical_device_format_properties(physical_device, *f);
                match tiling {
                    vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(features),
                    vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(features),
                    _ => false,
                }
            })
            .ok_or_else(|| anyhow!("Failed to find supported format"))
        }
    }
    
    pub fn is_texture_format_supported(
        instance: Instance,
        physical_device: vk::PhysicalDevice,
        format: vk::Format,
    ) -> bool {
        let properties = unsafe { instance.get_physical_device_format_properties(physical_device, format) };
        // Check if format is supported for optimal tiling with sampled image feature
        properties.optimal_tiling_features.contains(vk::FormatFeatureFlags::SAMPLED_IMAGE)
    }
    
    fn update_bindless_texture(device: Device, rp_engine: &RenderPipelineEngine, slot_index: u32, view: vk::ImageView, sampler: vk::Sampler) -> Result<()> {
        if rp_engine.descriptor_sets.is_empty() {
            return Ok(());
        }

        let info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(view)
            .sampler(sampler);

        let image_info = &[info];
        for descriptor_set in &rp_engine.descriptor_sets {
            let write_set = vk::WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(1)
                .dst_array_element(slot_index)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(image_info);

            unsafe { device.update_descriptor_sets(&[write_set], &[] as &[vk::CopyDescriptorSet]); }
        }
        Ok(())
    }

    fn transition_image_layout(
        context: DeviceContext,
        command_engine: CommandEngine,
        image: vk::Image,
        format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        mipmap_levels: u32,
    ) -> Result<()> {
        let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) = match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            _ => return Err(anyhow!("Unsupported image layout transition!")),
        };
    
        let command_buffer = command_engine.begin_single_time_commands(context.clone().device)?;
    
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
            context.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[] as &[vk::MemoryBarrier],
                &[] as &[vk::BufferMemoryBarrier],
                &[barrier],
            );
        }
    
        command_engine.end_single_time_commands(context, command_buffer)?;
    
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Texture {
    slot_index: u32,
    image: vk::Image,
    memory: vk::DeviceMemory,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    instance_count: u32,
}