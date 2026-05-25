#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::Result;
use vulkanalia::vk::DeviceV1_0;
use vulkanalia::prelude::v1_0::*;
use crate::components::render::Render;
use crate::ecs::World;
use crate::engine::texture_engine::SamplerContents;
use crate::engine::{CommandEngine, CommandEngineBuilder};
use crate::engine::DeviceContext;
use crate::engine::{ModelEngine, ModelEngineBuilder};
use crate::engine::{PresentEngine, PresentEngineBuilder};
use crate::engine::{RenderPipelineEngine, RenderPipelineEngineBuilder};
use crate::engine::TextureEngine;
use std::sync::Arc;

#[derive(Default)]
pub struct App {
    pub device_context: Arc<Option<DeviceContext>>,
    pub present_engine: Arc<PresentEngine>,
    pub rp_engine: Arc<RenderPipelineEngine>,
    pub command_engine: Arc<CommandEngine>,
    pub model_engine: Arc<ModelEngine>,
    pub texture_engine: Arc<TextureEngine>,
    pub world: World,
}

impl App {
    pub fn new(world: World) -> Result<Self> {
        let mut app = App::default();
        app.world = world;

        unsafe {
            // Create builders
            let mut present_engine_builder = PresentEngineBuilder::new();
            let mut rp_engine_builder = RenderPipelineEngineBuilder::new();
            let mut command_engine_builder = CommandEngineBuilder::new();
            let mut model_engine_builder = ModelEngineBuilder::new();
            
            // Create window
            present_engine_builder.create_window(true)?;
            
            // Create Device Context
            app.device_context = Arc::new(Some(DeviceContext::new(&mut present_engine_builder)?));

            // Set multisample antialiasing
            present_engine_builder.set_default_msaa(app.device_context.as_ref().clone().unwrap().instance, app.device_context.as_ref().clone().unwrap().physical_device);
        
            // Create swapchain
            present_engine_builder.create_swapchain(app.device_context.as_ref().clone().unwrap())?;

            // Create color image
            present_engine_builder.create_color_objects(app.device_context.as_ref().clone().unwrap())?;
        
            // Create depth image
            present_engine_builder.create_depth_objects(app.device_context.as_ref().clone().unwrap())?;

            // Create render pass
            rp_engine_builder.create_render_pass(app.device_context.as_ref().clone().unwrap(), present_engine_builder.0.clone())?;

            // Create a descriptor set layout for uniform buffer objects and texture samplers
            rp_engine_builder.create_descriptor_set_layout(app.device_context.as_ref().clone().unwrap().device)?;

            // Create a descriptor pool
            rp_engine_builder.create_descriptor_pool(app.device_context.as_ref().clone().unwrap().device, present_engine_builder.0.clone())?;

            // Create the render pipeline
            rp_engine_builder.create_pipeline(app.device_context.as_ref().clone().unwrap(), present_engine_builder.0.clone())?;

            // Create framebuffers
            rp_engine_builder.create_framebuffers(app.device_context.as_ref().clone().unwrap().device, present_engine_builder.0.clone())?;

            // Create command pool
            command_engine_builder.create_command_pool(app.device_context.as_ref().clone().unwrap())?;

            // Create uniform buffer objects
            model_engine_builder.create_uniform_buffers(app.device_context.as_ref().clone().unwrap(), present_engine_builder.0.clone())?;

            // Load texture
            Arc::make_mut(&mut app.texture_engine).load_texture(
                app.device_context.as_ref().clone().unwrap(),
                rp_engine_builder.0.clone(),
                command_engine_builder.0.clone(),
                "cuttlefish_albedo".to_string(),
                SamplerContents::new(
                    vk::Filter::LINEAR,
                    vk::SamplerAddressMode::REPEAT,
                    vk::SamplerAddressMode::REPEAT,
                    vk::SamplerAddressMode::REPEAT,
                    vk::SamplerMipmapMode::LINEAR,
                ),
            )?;

            // Load model
            model_engine_builder.0.load_model(app.device_context.as_ref().clone().unwrap(), command_engine_builder.0.clone(), "".to_string())?;

            // Create descriptor sets
            rp_engine_builder.create_descriptor_sets(
                app.device_context.as_ref().clone().unwrap().device,
                present_engine_builder.0.clone(),
                model_engine_builder.0.clone(),
                app.texture_engine.as_ref().clone(),
            )?;

            let texture_slot_index = app.texture_engine.as_ref().get_texture_slot_index(&app.world.query::<Render>().unwrap().0[0].material.albedo_name).unwrap_or_default();

            // Create command buffers
            command_engine_builder.create_command_buffers(
                app.device_context.as_ref().clone().unwrap().device,
                present_engine_builder.0.clone(),
                rp_engine_builder.0.clone(),
                model_engine_builder.0.clone(),
                texture_slot_index,
            )?;

            // Create sync objects
            command_engine_builder.create_sync_objects(app.device_context.as_ref().clone().unwrap().device, present_engine_builder.0.clone())?;

            // Make engines persistent
            app.present_engine = Arc::new(present_engine_builder.0);
            app.rp_engine = Arc::new(rp_engine_builder.0);
            app.command_engine = Arc::new(command_engine_builder.0);
            app.model_engine = Arc::new(model_engine_builder.0);
        }

        Ok(app)
    }

    pub fn run(&mut self) {
        PresentEngine::update_window(self).unwrap();
    }

    pub fn destroy(&mut self) {
        let device = self.device_context.as_ref().clone().unwrap().device;
        unsafe { device.device_wait_idle().unwrap(); }
        Arc::make_mut(&mut self.present_engine).destroy(device.clone());
        Arc::make_mut(&mut self.rp_engine).destroy(device.clone());
        Arc::make_mut(&mut self.command_engine).destroy(device.clone());
        Arc::make_mut(&mut self.model_engine).destroy(device.clone());
        Arc::make_mut(&mut self.texture_engine).destroy(device.clone());
    }
}