#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use vulkanalia::bytecode::Bytecode;
use vulkanalia::prelude::v1_0::*;
use std::mem::size_of;

use crate::engine::{CommandEngine, ModelEngine, PresentEngine, QuantizedVertex, TextureEngine, UniformBufferObject};
use super::device_context::DeviceContext;

const BINDLESS_TEXTURE_COUNT: u32 = 10_000;

#[derive(Clone, Default)]
pub struct RenderPipelineEngine {
    pub render_pass: vk::RenderPass,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl RenderPipelineEngine {
    pub fn destroy(&mut self, device: Device) {
        unsafe {
            self.framebuffers.iter().for_each(|f| device.destroy_framebuffer(*f, None));
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_render_pass(self.render_pass, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }

    pub fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
        unsafe {
            let bytecode = Bytecode::new(bytecode).unwrap();
            let info = vk::ShaderModuleCreateInfo::builder()
                .code(bytecode.code())
                .code_size(bytecode.code_size());
            Ok(device.create_shader_module(&info, None)?)
        }
    }
}

pub struct RenderPipelineEngineBuilder(pub(crate) RenderPipelineEngine);

impl RenderPipelineEngineBuilder {
    pub fn new() -> Self {
        Self(RenderPipelineEngine::default())
    }

    pub unsafe fn create_render_pass(&mut self, context: DeviceContext, present_engine: PresentEngine) -> Result<()> {
        // Attachments
    
        let color_attachment = vk::AttachmentDescription::builder()
            .format(present_engine.swapchain_format)
            .samples(present_engine.msaa_samples)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    
        let depth_stencil_attachment = vk::AttachmentDescription::builder()
            .format(PresentEngine::get_depth_format(context.instance, context.physical_device)?)
            .samples(present_engine.msaa_samples)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    
        let color_resolve_attachment = vk::AttachmentDescription::builder()
            .format(present_engine.swapchain_format)
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
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);
    
        // Create
    
        let attachments = &[color_attachment, depth_stencil_attachment, color_resolve_attachment];
        let subpasses = &[subpass];
        let dependencies = &[dependency];
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(subpasses)
            .dependencies(dependencies);
    
        self.0.render_pass = context.device.create_render_pass(&info, None)?;
    
        Ok(())
    }    

    pub unsafe fn create_descriptor_set_layout(&mut self, device: Device) -> Result<()> {
        let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
    
        let texture_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(1)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(BINDLESS_TEXTURE_COUNT)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(6)
            .descriptor_type(vk::DescriptorType::SAMPLER)
            .descriptor_count(BINDLESS_TEXTURE_COUNT)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let static_model_matrix_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let dyn_model_matrix_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(3)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let indirect_draw_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(4)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let instance_data_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(5)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let binding_flags = &[
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::empty(),
            vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        ];
        let mut layout_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
            .binding_flags(binding_flags);

        let bindings = &[ubo_binding, texture_binding, static_model_matrix_binding, dyn_model_matrix_binding, indirect_draw_binding, instance_data_binding, sampler_binding];
        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(bindings)
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .push_next(&mut layout_flags);

        self.0.descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;
    
        Ok(())
    }

    pub unsafe fn create_descriptor_pool(&mut self, device: Device, present_engine: PresentEngine, command_engine: CommandEngine) -> Result<()> {
        let ubo_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(command_engine.max_frames_in_flight as u32);

        let static_model_matrix_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(command_engine.max_frames_in_flight as u32);

        let dyn_model_matrix_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(command_engine.max_frames_in_flight as u32);

        let indirect_draw_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(command_engine.max_frames_in_flight as u32);

        let instance_data_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(command_engine.max_frames_in_flight as u32);
    
        let texture_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(BINDLESS_TEXTURE_COUNT * present_engine.swapchain_images.len() as u32);

        let sampler_size = vk::DescriptorPoolSize::builder()
            .type_(vk::DescriptorType::SAMPLER)
            .descriptor_count(BINDLESS_TEXTURE_COUNT * present_engine.swapchain_images.len() as u32);
    
        let pool_sizes = &[ubo_size, texture_size, static_model_matrix_size, dyn_model_matrix_size, indirect_draw_size, instance_data_size, sampler_size];
        let info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .pool_sizes(pool_sizes)
            .max_sets(command_engine.max_frames_in_flight as u32);
    
        self.0.descriptor_pool = device.create_descriptor_pool(&info, None)?;
    
        Ok(())
    }

    pub unsafe fn create_descriptor_sets(&mut self, device: Device, model_engine: ModelEngine, command_engine: CommandEngine, texture_engine: TextureEngine) -> Result<()> {
        // Allocate
    
        let layouts = vec![self.0.descriptor_set_layout; command_engine.max_frames_in_flight];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.0.descriptor_pool)
            .set_layouts(&layouts);
    
        self.0.descriptor_sets = device.allocate_descriptor_sets(&info)?;
    
        // Update
    
        for i in 0..command_engine.max_frames_in_flight {
            let info = vk::DescriptorBufferInfo::builder()
                .buffer(model_engine.uniform_buffers[i])
                .offset(0)
                .range(size_of::<UniformBufferObject>() as u64);
    
            let buffer_info = &[info];
            let ubo_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.0.descriptor_sets[i])
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(buffer_info);

            let static_model_matrix_info = vk::DescriptorBufferInfo::builder()
                .buffer(model_engine.static_model_matrix_buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let static_model_matrix_buffer_info = [static_model_matrix_info];
            let static_model_matrix_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.0.descriptor_sets[i])
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&static_model_matrix_buffer_info);

            let dyn_model_matrix_info = vk::DescriptorBufferInfo::builder()
                .buffer(model_engine.dyn_model_matrix_buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let dyn_model_matrix_buffer_info = [dyn_model_matrix_info];
            let dyn_model_matrix_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.0.descriptor_sets[i])
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&dyn_model_matrix_buffer_info);

            let indirect_draw_info = vk::DescriptorBufferInfo::builder()
                .buffer(command_engine.indirect_draw_buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let indirect_draw_buffer_info = [indirect_draw_info];
            let indirect_draw_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.0.descriptor_sets[i])
                .dst_binding(4)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&indirect_draw_buffer_info);

            let instance_data_info = vk::DescriptorBufferInfo::builder()
                .buffer(command_engine.instance_buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let instance_data_buffer_info = [instance_data_info];
            let instance_data_write = vk::WriteDescriptorSet::builder()
                .dst_set(self.0.descriptor_sets[i])
                .dst_binding(5)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&instance_data_buffer_info);

            device.update_descriptor_sets(&[ubo_write, static_model_matrix_write, dyn_model_matrix_write, indirect_draw_write, instance_data_write], &[] as &[vk::CopyDescriptorSet]);
        }

        texture_engine.refresh_bindless_textures(device, &self.0)?;
    
        Ok(())
    }

    pub unsafe fn create_pipeline(&mut self, context: DeviceContext, present_engine: PresentEngine) -> Result<()> {
        // Stages
    
        let shader = include_bytes!("../../assets/shaders/shader.spv");
        
        let shader_module = RenderPipelineEngine::create_shader_module(&context.device, &shader[..])?;
    
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
        let attribute_descriptions = QuantizedVertex::attribute_descriptions(&context)?;
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
            .width(present_engine.swapchain_extent.width as f32)
            .height(present_engine.swapchain_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);
    
        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(present_engine.swapchain_extent);
    
        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors);
    
        // Rasterization State
    
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_bias_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);
    
        // Multisample State
    
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(true)
            .min_sample_shading(0.2)
            .rasterization_samples(present_engine.msaa_samples);
    
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
    
        let set_layouts = &[self.0.descriptor_set_layout];
        let push_constant_ranges = &[vk::PushConstantRange::builder()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(size_of::<PushConstant>() as u32)
            .build()];
        let layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts)
            .push_constant_ranges(push_constant_ranges);
        self.0.pipeline_layout = context.device.create_pipeline_layout(&layout_info, None)?;
    
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
            .layout(self.0.pipeline_layout)
            .render_pass(self.0.render_pass)
            .subpass(0);
    
        self.0.pipeline = context.device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
            .0[0];
    
        // Cleanup
    
        context.device.destroy_shader_module(shader_module, None);
    
        Ok(())
    }

    pub unsafe fn create_framebuffers(&mut self, device: Device, present_engine: PresentEngine) -> Result<()> {
        self.0.framebuffers = present_engine
            .swapchain_image_views
            .iter()
            .map(|i| {
                let attachments = &[present_engine.color_image_view, present_engine.depth_image_view, *i];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(self.0.render_pass)
                    .attachments(attachments)
                    .width(present_engine.swapchain_extent.width)
                    .height(present_engine.swapchain_extent.height)
                    .layers(1);

                device.create_framebuffer(&create_info, None)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PushConstant {
    pub model_matrix_info: u32,
    pub texture_index: u32,
}