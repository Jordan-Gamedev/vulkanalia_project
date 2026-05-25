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
    pub fn new() -> Result<Box<Self>> {
        let mut app = Box::new(App::default());
        app.world.app = &mut *app as *mut App;

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
            rp_engine_builder.create_descriptor_pool(app.device_context.as_ref().clone().unwrap().device, present_engine_builder.0.clone(), command_engine_builder.0.clone())?;

            // Create the render pipeline
            rp_engine_builder.create_pipeline(app.device_context.as_ref().clone().unwrap(), present_engine_builder.0.clone())?;

            // Create framebuffers
            rp_engine_builder.create_framebuffers(app.device_context.as_ref().clone().unwrap().device, present_engine_builder.0.clone())?;

            // Create command pool
            command_engine_builder.create_command_pool(app.device_context.as_ref().clone().unwrap())?;

            // Create uniform buffer objects
            model_engine_builder.create_uniform_buffers(app.device_context.as_ref().clone().unwrap(), command_engine_builder.0.clone())?;

            // Create descriptor sets
            rp_engine_builder.create_descriptor_sets(
                app.device_context.as_ref().clone().unwrap().device,
                model_engine_builder.0.clone(),
                command_engine_builder.0.clone(),
                app.texture_engine.as_ref().clone(),
            )?;

            // Create command buffers
            command_engine_builder.create_command_buffers(
                app.device_context.as_ref().clone().unwrap().device,
                present_engine_builder.0.clone(),
                rp_engine_builder.0.clone(),
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

    pub fn load_model(&mut self, path: String) -> Result<()> {
        Arc::make_mut(&mut self.model_engine).load_model(self.device_context.as_ref().clone().unwrap(), self.command_engine.as_ref().clone(), path)?;
        Ok(())
    }

    pub fn load_texture(&mut self, path: String, sampler_contents: SamplerContents) -> Result<()> {
        Arc::make_mut(&mut self.texture_engine).load_texture(self.device_context.as_ref().clone().unwrap(), self.rp_engine.as_ref().clone(), self.command_engine.as_ref().clone(), path, sampler_contents)?;
        Ok(())
    }

    pub fn unload_texture(&mut self, path: String) -> Result<()> {
        Arc::make_mut(&mut self.texture_engine).unload_texture(self.device_context.as_ref().clone().unwrap(), self.rp_engine.as_ref().clone(), path)?;
        Ok(())
    }

    pub fn unload_model(&mut self, path: String) -> Result<()> {
        Arc::make_mut(&mut self.model_engine).unload_model(self.device_context.as_ref().clone().unwrap(), self.command_engine.as_ref().clone(), path)?;
        Ok(())
    }
}