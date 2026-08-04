#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::{self, Result};
use std::sync::Arc;
use vulkanalia::vk::{KhrSurfaceExtensionInstanceCommands, KhrSwapchainExtensionDeviceCommands};
use log::info;
use vulkanalia::prelude::v1_0::*;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Fullscreen, Window, WindowBuilder};

use crate::engine::{App, CommandEngine, CommandEngineBuilder, DeviceContext, ModelEngineBuilder, RenderPipelineEngineBuilder, TextureEngine};


#[derive(Clone, Default)]
pub struct PresentEngine {
    // Window
    pub window: Option<Arc<Window>>,
    pub event_loop: Option<Arc<EventLoop<()>>>,
    pub surface: vk::SurfaceKHR,
    // Swapchain
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    // Color
    pub color_image: vk::Image,
    pub color_image_memory: vk::DeviceMemory,
    pub color_image_view: vk::ImageView,
    // Depth
    pub depth_image: vk::Image,
    pub depth_image_memory: vk::DeviceMemory,
    pub depth_image_view: vk::ImageView,
    // Msaa
    pub msaa_samples: vk::SampleCountFlags,
    // Other
    pub resized: bool,
}

impl PresentEngine {
    pub fn destroy(&mut self, device: Device) {
        unsafe {
            device.destroy_image_view(self.depth_image_view, None);
            device.destroy_image(self.depth_image, None);
            device.free_memory(self.depth_image_memory, None);
            device.destroy_image_view(self.color_image_view, None);
            device.destroy_image(self.color_image, None);
            device.free_memory(self.color_image_memory, None);
            self.swapchain_image_views.iter().for_each(|v| device.destroy_image_view(*v, None));
            device.destroy_swapchain_khr(self.swapchain, None);
        }
    }

    pub fn update_window(app: &mut App, bevy_app: &mut bevy_app::App) -> Result<()> {
        let event_loop = Arc::into_inner(
            Arc::make_mut(&mut app.present_engine).event_loop.take().unwrap()
        ).unwrap();

        event_loop.run(move |event, elwt| {
            match event {
                // Request a redraw after all events are processed
                Event::AboutToWait => app.present_engine.as_ref().window.as_ref().unwrap().request_redraw(),
                Event::WindowEvent { event, .. } => match event {
                    
                    // Render a frame if the Vulkan app is not being destroyed
                    WindowEvent::RedrawRequested if !elwt.exiting() => {
                        // TODO: Jolt physics should update here as well
                        bevy_app.update();
                        CommandEngine::render(app).unwrap();
                    },
                    
                    // Mark the window as having been resized
                    WindowEvent::Resized(size) => {
                        Arc::make_mut(&mut app.present_engine).resized = true;
                    },
    
                    // Destroy the Vulkan app
                    WindowEvent::CloseRequested => {
                        Arc::make_mut(&mut app.present_engine).window.as_mut().unwrap().set_visible(false);
                        app.destroy();
                        elwt.exit();
                    },
                    _ => {},
                }
                _ => {}
            }
        })?;

        Ok(())
    }

    /// Recreates the swapchain for the Vulkan app
    #[rustfmt::skip]
    pub fn recreate_swapchain(app: &mut App) -> Result<()> {
        unsafe {
            let size = app.present_engine.window.as_ref().unwrap().inner_size();
            if size.width == 0 || size.height == 0 {
                return Ok(());
            }

            app.device_context.as_ref().clone().unwrap().device.device_wait_idle()?;
            PresentEngine::destroy_swapchain(app);
            
            // Update presentation
            let mut present_engine_builder = PresentEngineBuilder::new();
            present_engine_builder.0 = app.present_engine.as_ref().clone();
            present_engine_builder.create_swapchain(app.device_context.as_ref().clone().unwrap())?;
            present_engine_builder.create_color_objects(app.device_context.as_ref().clone().unwrap())?;
            present_engine_builder.create_depth_objects(app.device_context.as_ref().clone().unwrap())?;
            app.present_engine = present_engine_builder.0.into();

            // Update ubos
            let mut model_engine_builder = ModelEngineBuilder::new();
            model_engine_builder.0 = app.model_engine.as_ref().clone();
            model_engine_builder.create_uniform_buffers(
                app.device_context.as_ref().clone().unwrap(),
                app.command_engine.as_ref().clone(),
            )?;
            app.model_engine = model_engine_builder.0.into();

            // Update render pipeline
            let mut render_engine_builder = RenderPipelineEngineBuilder::new();
            render_engine_builder.0 = app.rp_engine.as_ref().clone();
            render_engine_builder.create_render_pass(
                app.device_context.as_ref().clone().unwrap(),
                app.present_engine.as_ref().clone(),
            )?;
            render_engine_builder.create_descriptor_pool(
                app.device_context.as_ref().clone().unwrap().device,
                app.present_engine.as_ref().clone(),
                app.command_engine.as_ref().clone(),
            )?;
            render_engine_builder.create_pipeline(
                app.device_context.as_ref().clone().unwrap(),
                app.present_engine.as_ref().clone(),
            )?;
            render_engine_builder.create_framebuffers(
                app.device_context.as_ref().clone().unwrap().device,
                app.present_engine.as_ref().clone(),
            )?;

            // Finish updating render pipeline
            render_engine_builder.create_descriptor_sets(
                app.device_context.as_ref().clone().unwrap().device,
                app.model_engine.as_ref().clone(),
                app.command_engine.as_ref().clone(),
                app.texture_engine.as_ref().clone(),
            )?;
            app.rp_engine = render_engine_builder.0.into();

            // Update command buffers
            let mut command_engine_builder = CommandEngineBuilder::new();
            command_engine_builder.0 = app.command_engine.as_ref().clone();
                command_engine_builder.create_command_buffers(
                    app.device_context.as_ref().clone().unwrap().device,
                    app.present_engine.as_ref().clone(),
                    app.rp_engine.as_ref().clone(),
            )?;
            command_engine_builder.0.images_in_flight.resize(app.present_engine.swapchain_images.len(), vk::Fence::null());
            app.command_engine = command_engine_builder.0.into();
            
            Ok(())
        }
    }

    /// Destroys the parts of our Vulkan app related to the swapchain
    #[rustfmt::skip]
    fn destroy_swapchain(app: &mut App) {
        unsafe {
            let device = app.device_context.as_ref().clone().unwrap().device;
            if app.command_engine.command_pool != vk::CommandPool::null() && !app.command_engine.command_buffers.is_empty() {
                device.free_command_buffers(app.command_engine.command_pool, &app.command_engine.command_buffers);
            }
            device.destroy_descriptor_pool(app.rp_engine.descriptor_pool, None);
            device.destroy_image_view(app.present_engine.depth_image_view, None);
            device.destroy_image(app.present_engine.depth_image, None);
            device.free_memory(app.present_engine.depth_image_memory, None);
            device.destroy_image_view(app.present_engine.color_image_view, None);
            device.destroy_image(app.present_engine.color_image, None);
            device.free_memory(app.present_engine.color_image_memory, None);
            app.rp_engine.framebuffers.iter().for_each(|f| device.destroy_framebuffer(*f, None));
            device.destroy_pipeline(app.rp_engine.pipeline, None);
            device.destroy_pipeline_layout(app.rp_engine.pipeline_layout, None);
            device.destroy_render_pass(app.rp_engine.render_pass, None);
            app.present_engine.swapchain_image_views.iter().for_each(|v| device.destroy_image_view(*v, None));
            device.destroy_swapchain_khr(app.present_engine.swapchain, None);
        }
    }

    pub fn get_max_msaa_samples(instance: Instance, physical_device: vk::PhysicalDevice) -> vk::SampleCountFlags {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let counts = properties.limits.framebuffer_color_sample_counts & properties.limits.framebuffer_depth_sample_counts;
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

    unsafe fn get_swapchain_surface_format(context: &DeviceContext, surface: vk::SurfaceKHR) -> vk::SurfaceFormatKHR {      
        let formats = context.instance.get_physical_device_surface_formats_khr(context.physical_device, surface).unwrap();
        let format = formats
            .iter()
            .cloned()
            .find(|f| (f.format == vk::Format::B8G8R8_SRGB || f.format == vk::Format::R8G8B8_SRGB) 
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .or_else(|| formats
                .iter()
                .cloned()
                .find(|f| f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR))
            .unwrap_or_else(|| formats[0]);
        
        info!("Selected swapchain format: {:?}, color space: {:?}", format.format, format.color_space);
        format
    }

    unsafe fn get_swapchain_present_mode(context: &DeviceContext, surface: vk::SurfaceKHR) -> vk::PresentModeKHR {
        let present_modes = context.instance.get_physical_device_surface_present_modes_khr(context.physical_device, surface).unwrap();
        
        present_modes
            .iter()
            .cloned()
        .find(|m| *m == vk::PresentModeKHR::IMMEDIATE)
        .or_else(|| present_modes.iter().cloned().find(|m| *m == vk::PresentModeKHR::MAILBOX))
        .unwrap_or(vk::PresentModeKHR::FIFO)
    }
    
    #[rustfmt::skip]
    fn get_swapchain_extent(window: &Window, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
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

    pub fn get_depth_format(instance: Instance, physical_device: vk::PhysicalDevice) -> Result<vk::Format> {
        let candidates = &[
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
        ];
    
        TextureEngine::get_supported_format(
            instance,
            physical_device,
            candidates,
            vk::ImageTiling::OPTIMAL,
            vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }
}

pub struct PresentEngineBuilder(pub(crate) PresentEngine);

impl PresentEngineBuilder {
    pub fn new() -> Self {
        Self(PresentEngine::default())
    }

    pub unsafe fn create_window(&mut self, with_fullscreen: bool) -> Result<()> {
        // On Linux, winit expects WAYLAND_DISPLAY, WAYLAND_SOCKET or DISPLAY to be set.
        // If the environment doesn't provide any of these, try to detect a common
        // Wayland socket location (e.g. $XDG_RUNTIME_DIR/wayland-0 or /run/wayland-0)
        // and set `WAYLAND_DISPLAY=wayland-0` so winit can connect when appropriate.
        PresentEngineBuilder::ensure_wayland_env();

        // Window

        self.0.event_loop = Some(Arc::new(EventLoop::new()?));
        
        self.0.window = Some(Arc::new(WindowBuilder::new()
            .with_title("Vulkanalia Game")
            .with_inner_size(LogicalSize::new(2560, 1600))
            .build(&self.0.event_loop.as_ref().unwrap())
            .unwrap()));

        // Set fullscreen if enabled
        if with_fullscreen && let Some(monitor) = self.0.window.as_ref().unwrap().current_monitor().or_else(|| self.0.window.as_ref().unwrap().primary_monitor()) {
            if let Some(video_mode) = monitor
                .video_modes()
                //.max_by_key(|mode| mode.refresh_rate_millihertz() + mode.size().width * mode.size().height)
                .find(|mode| {
                    mode.refresh_rate_millihertz() / 1000 == 240 &&
                    mode.size().width == 1920 &&
                    mode.size().height == 1200
                })
            {
                self.0.window.as_mut().unwrap().set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())));

                println!("\nDisplay: {}x{}@{}Hz\n", video_mode.size().width, video_mode.size().height, video_mode.refresh_rate_millihertz() / 1000);
            }
        }

        Ok(())
    }

    pub unsafe fn create_surface(&mut self, instance: Instance) -> Result<()> {
        // Surface
        self.0.surface = vulkanalia::window::create_surface(&instance, self.0.window.as_ref().unwrap(), &self.0.window.as_ref().unwrap())?;
        Ok(())
    }

    pub unsafe fn set_default_msaa(&mut self, instance: Instance, physical_device: vk::PhysicalDevice) {
        let max_msaa = PresentEngine::get_max_msaa_samples(instance, physical_device);
        let chosen_msaa = if max_msaa < vk::SampleCountFlags::_4 { max_msaa } else { vk::SampleCountFlags::_4 }; 
        info!("Max msaa detected: {:?}", max_msaa);
        info!("Chosen msaa: {:?}", chosen_msaa);
        self.0.msaa_samples = chosen_msaa;
    }

    pub unsafe fn create_swapchain(&mut self, context: DeviceContext) -> Result<()> {
        let window = self.0.window.as_ref().unwrap();

        // Image
    
        let swapchain_capabilities = context.instance.get_physical_device_surface_capabilities_khr(context.physical_device, self.0.surface)?;
        
        let surface_format = PresentEngine::get_swapchain_surface_format(&context, self.0.surface);
        let present_mode = PresentEngine::get_swapchain_present_mode(&context, self.0.surface);
        let extent = PresentEngine::get_swapchain_extent(window, swapchain_capabilities);
    
        self.0.swapchain_format = surface_format.format;
        self.0.swapchain_extent = extent;
    
        let mut image_count = swapchain_capabilities.min_image_count + 1;
        if swapchain_capabilities.max_image_count != 0 && image_count > swapchain_capabilities.max_image_count {
            image_count = swapchain_capabilities.max_image_count
        }
    
        let mut queue_family_indices = vec![];
        let image_sharing_mode = if context.graphics_queue_family_index != context.present_queue_family_index {
            queue_family_indices.push(context.graphics_queue_family_index);
            queue_family_indices.push(context.present_queue_family_index);
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };
    
        // Create
    
        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(self.0.surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(image_sharing_mode)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(swapchain_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());
    
            self.0.swapchain = context.device.create_swapchain_khr(&info, None)?;
    
        // Images
    
        self.0.swapchain_images = context.device.get_swapchain_images_khr(self.0.swapchain)?;
    
        // Image Views

        self.0.swapchain_image_views = self.0
            .swapchain_images
            .iter()
            .map(|i| TextureEngine::create_image_view(context.clone().device, *i, self.0.swapchain_format, vk::ImageAspectFlags::COLOR, 1))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }
    
    pub unsafe fn create_color_objects(&mut self, context: DeviceContext) -> Result<()> {
        // Image + Image Memory
    
        let (color_image, color_image_memory) = TextureEngine::create_image(
            context.clone(),
            self.0.swapchain_extent.width,
            self.0.swapchain_extent.height,
            1,
            self.0.msaa_samples,
            self.0.swapchain_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL
        )?;
    
        self.0.color_image = color_image;
        self.0.color_image_memory = color_image_memory;
    
        // Image View
    
        self.0.color_image_view = TextureEngine::create_image_view(
            context.device,
            self.0.color_image,
            self.0.swapchain_format,
            vk::ImageAspectFlags::COLOR,
            1,
        )?;
    
        Ok(())
    }
    
    pub unsafe fn create_depth_objects(&mut self, context: DeviceContext) -> Result<()> {
        // Image + Image Memory
    
        let format = PresentEngine::get_depth_format(context.clone().instance, context.clone().physical_device)?;
    
        let (depth_image, depth_image_memory) = TextureEngine::create_image(
            context.clone(),
            self.0.swapchain_extent.width,
            self.0.swapchain_extent.height,
            1,
            self.0.msaa_samples,
            format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
    
        self.0.depth_image = depth_image;
        self.0.depth_image_memory = depth_image_memory;
    
        // Image view
    
        self.0.depth_image_view = TextureEngine::create_image_view(context.clone().device, self.0.depth_image, format, vk::ImageAspectFlags::DEPTH, 1)?;
    
        Ok(())
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

}