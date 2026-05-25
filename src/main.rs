use vulkanalia_project::components::render::{Material, Render};
use vulkanalia_project::ecs::{World};
use vulkanalia_project::engine::App;

fn main() {
    let mut world = World::new();
    let entity = world.create_entity();
    let render_component = Render {
        model_matrix_index: 0,
        model_name: "Limpet".to_string(),
        material: Material {
            sampler_index: 0,
            albedo_name: "cuttlefish_albedo".to_string(),
            normal_ao_name: String::new(),
            metallic_roughness_emissive_name: String::new(),
        },
        is_receiving_shadows: true,
        is_casting_shadows: true,
    };
    world.add_component(entity, render_component);

    let mut app = App::new(world).unwrap();

    // Run the app
    app.run();
}

// #![allow(
//     dead_code,
//     unsafe_op_in_unsafe_fn,
//     unused_variables,
//     clippy::manual_slice_size_calculation,
//     clippy::too_many_arguments,
//     clippy::unnecessary_wraps
// )]

// // Prevent terminal from appearing on release builds for Windows
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// use anyhow::{anyhow, Result};
// use glam::{Mat4, Vec2, Vec3, vec2, vec3};
// use log::*;
// use std::collections::HashSet;
// use std::f32::consts::PI;
// use std::ffi::CStr;
// use std::fmt::Debug;
// use std::mem::size_of;
// use std::os::raw::c_void;
// use std::ptr::copy_nonoverlapping as memcpy;
// use std::time::Instant;
// use thiserror::Error;
// use vulkanalia::bytecode::Bytecode;
// use vulkanalia::loader::{LIBRARY, LibloadingLoader};
// use vulkanalia::prelude::v1_0::*;
// use vulkanalia::Version;
// use vulkanalia::vk::ExtDebugUtilsExtensionInstanceCommands;
// use vulkanalia::vk::KhrSurfaceExtensionInstanceCommands;
// use vulkanalia::vk::KhrSwapchainExtensionDeviceCommands;
// use vulkanalia::window as vk_window;
// use winit::dpi::LogicalSize;
// use winit::event::{Event, WindowEvent};
// use winit::event_loop::EventLoop;
// use winit::window::{Fullscreen, Window, WindowBuilder};

// /// Whether the validation layers should be enabled (only enabled if debug assertions flag is active)
// const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

// /// The name of the validation layers
// const VALIDATION_LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

// /// The required device extensions.
// const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

// /// The Vulkan SDK version that started requiring the portability subset extension for macOS.
// const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

// /// The maximum number of frames that can be processed concurrently
// const MAX_FRAMES_IN_FLIGHT: usize = 2;

// const DEG_TO_RAD: f32 = PI / 180.0;

// #[cfg(target_os = "linux")]
// unsafe fn ensure_wayland_env() {
//     use std::env;
//     use std::path::Path;

//     if env::var_os("WAYLAND_DISPLAY").is_none()
//         && env::var_os("WAYLAND_SOCKET").is_none()
//         && env::var_os("DISPLAY").is_none()
//     {
//         if let Some(xdg) = env::var_os("XDG_RUNTIME_DIR") {
//             let p = Path::new(&xdg).join("wayland-0");
//             if p.exists() {
//                 env::set_var("WAYLAND_DISPLAY", "wayland-0");
//                 return;
//             }
//         }

//         let candidates = ["/run/user/1000/wayland-0", "/run/wayland-0"];
//         for c in candidates {
//             if Path::new(c).exists() {
//                 env::set_var("WAYLAND_DISPLAY", "wayland-0");
//                 return;
//             }
//         }
//     }
// }

// #[cfg(not(target_os = "linux"))]
// unsafe fn ensure_wayland_env() {}

// #[rustfmt::skip]
// fn main() -> Result<()> {

//     // On Linux, winit expects WAYLAND_DISPLAY, WAYLAND_SOCKET or DISPLAY to be set.
//     // If the environment doesn't provide any of these, try to detect a common
//     // Wayland socket location (e.g. $XDG_RUNTIME_DIR/wayland-0 or /run/wayland-0)
//     // and set `WAYLAND_DISPLAY=wayland-0` so winit can connect when appropriate.
//     unsafe { ensure_wayland_env() };


//     #[cfg(debug_assertions)]
//     {
//         // Dev-only logger with a sensible default; RUST_LOG still overrides this.
//         let mut logger = pretty_env_logger::formatted_builder();
//         logger.parse_filters("info");
//         logger.parse_default_env();
//         logger.init();
//     }

//     // Window

//     let event_loop = EventLoop::new()?;
//     let window = create_window(&event_loop, true)?;

//     // App

//     let mut app = unsafe { App::create(&window)? };
//     event_loop.run(move |event, elwt| {
//         match event {
//             // Request a redraw after all events are processed
//             Event::AboutToWait => window.request_redraw(),
//             Event::WindowEvent { event, .. } => match event {
                
//                 // Render a frame if the Vulkan app is not being destroyed
//                 WindowEvent::RedrawRequested if !elwt.exiting() => unsafe { app.render(&window) }.unwrap(),
                
//                 // Mark the window as having been resized
//                 WindowEvent::Resized(size) => {
//                     app.resized = true;
//                 }

//                 // Destroy the Vulkan app
//                 WindowEvent::CloseRequested => {
//                     elwt.exit();
//                     unsafe { app.destroy(); }
//                 }
//                 _ => {}
//             }
//             _ => {}
//         }
//     })?;

//     fn create_window(event_loop: &EventLoop<()>, with_fullscreen: bool) -> Result<Window> {
//         let window = WindowBuilder::new()
//             .with_title("Vulkanalia Game")
//             .with_inner_size(LogicalSize::new(2560, 1600))
//             .build(event_loop)?;

//         if with_fullscreen && let Some(monitor) = window.current_monitor().or_else(|| window.primary_monitor()) {
//             if let Some(video_mode) = monitor
//                 .video_modes()
//                 .max_by_key(|mode| (mode.refresh_rate_millihertz(), mode.size().width * mode.size().height))
//             {
//                 //window.set_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
//                 window.set_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
//             }
//         }

//         Ok(window)
//     }

//     Ok(())
// }

// /// The Vulkan app
// #[derive(Clone, Debug)]
// struct App {
//     entry: Entry,
//     instance: Instance,
//     device: Device,
//     data: AppData,
//     frame: usize,
//     resized: bool,
//     start: Instant,
// }

// impl App {
//     /// Creates the Vulkan app
//     unsafe fn create(window: &Window) -> Result<Self> {
//         let loader = LibloadingLoader::new(LIBRARY)?;
//         let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
//         let mut data = AppData::default();
//         let instance = create_instance(window, &entry, &mut data)?;
//         data.surface = vk_window::create_surface(&instance, &window, &window)?;
//         pick_physical_device(&instance, &mut data)?;
//         let device = create_logical_device(&entry, &instance, &mut data)?;
//         create_swapchain(window, &instance, &device, &mut data)?;
//         create_swapchain_image_views(&device, &mut data)?;
//         create_render_pass(&instance, &device, &mut data)?;
//         create_descriptor_set_layout(&device, &mut data)?;
//         create_pipeline(&instance, &device, &mut data)?;
//         create_command_pool(&instance, &device, &mut data)?;
//         create_color_objects(&instance, &device, &mut data)?;
//         create_depth_objects(&instance, &device, &mut data)?;
//         create_framebuffers(&device, &mut data)?;
//         create_texture_image(&instance, &device, &mut data)?;
//         let (vertices, indices) = load_model(&mut data)?;
//         data.vertices.extend(vertices);
//         data.indices.extend(indices);
//         create_vertex_buffer(&instance, &device, &mut data)?;
//         create_index_buffer(&instance, &device, &mut data)?;
//         create_uniform_buffers(&instance, &device, &mut data)?;
//         create_descriptor_pool(&device, &mut data)?;
//         create_descriptor_sets(&device, &mut data)?;
//         create_command_buffers(&device, &mut data)?;
//         create_sync_objects(&device, &mut data)?;
//         Ok(Self {
//             entry,
//             instance,
//             data,
//             device,
//             frame: 0,
//             resized: false,
//             start: Instant::now(),
//         })
//     }

//     /// Renders a frame for the Vulkan app
//     unsafe fn render(&mut self, window: &Window) -> Result<()> {

//         let size = window.inner_size();
//         if size.width == 0 || size.height == 0 {
//             return Ok(());
//         }

//         let in_flight_fence = self.data.in_flight_fences[self.frame];

//         self.device.wait_for_fences(&[in_flight_fence], true, u64::MAX)?;
        
//         let result = self.device.acquire_next_image_khr(
//             self.data.swapchain,
//             u64::MAX,
//             self.data.image_available_semaphores[self.frame],
//             vk::Fence::null(),
//         );

//         let image_index = match result {
//             Ok((image_index, _)) => image_index as usize,
//             Err(vk::ErrorCode::OUT_OF_DATE_KHR) => return self.recreate_swapchain(window),
//             Err(e) => return Err(anyhow!(e)),
//         };

//         let image_in_flight = self.data.images_in_flight[image_index];
//         if !image_in_flight.is_null() {
//             self.device.wait_for_fences(&[image_in_flight], true, u64::MAX)?;
//         }

//         self.data.images_in_flight[image_index] = in_flight_fence;

//         self.update_uniform_buffer(image_index)?;

//         let wait_semaphores = &[self.data.image_available_semaphores[self.frame]];
//         let wait_stages = &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
//         let command_buffers = &[self.data.command_buffers[image_index]];
//         let signal_semaphores = &[self.data.render_finished_semaphores[image_index]];
//         let submit_info = vk::SubmitInfo::builder()
//             .wait_semaphores(wait_semaphores)
//             .wait_dst_stage_mask(wait_stages)
//             .command_buffers(command_buffers)
//             .signal_semaphores(signal_semaphores);

//         self.device.reset_fences(&[in_flight_fence])?;

//         self.device.queue_submit(self.data.graphics_queue, &[submit_info], in_flight_fence)?;

//         let swapchains = &[self.data.swapchain];
//         let image_indices = &[image_index as u32];
//         let present_info = vk::PresentInfoKHR::builder()
//             .wait_semaphores(signal_semaphores)
//             .swapchains(swapchains)
//             .image_indices(image_indices);

//         let result = self.device.queue_present_khr(self.data.present_queue, &present_info);

//         let changed = result == Ok(vk::SuccessCode::SUBOPTIMAL_KHR)
//             || result == Err(vk::ErrorCode::OUT_OF_DATE_KHR);

//         if self.resized || changed {
//             self.resized = false;
//             self.recreate_swapchain(window)?;
//         } else if let Err(e) = result {
//             return Err(anyhow!(e));
//         }

//         self.frame = (self.frame + 1) % MAX_FRAMES_IN_FLIGHT;

//         Ok(())
//     }

//     /// Updates the uniform buffer object for the Vulkan app
//     unsafe fn update_uniform_buffer(&self, image_index: usize) -> Result<()> {
//         // MVP

//         let time = self.start.elapsed().as_secs_f32();

//         let model = Mat4::from_axis_angle(vec3(0.0, 1.0, 0.0), 90.0 * DEG_TO_RAD * time);
    
//         let view = Mat4::look_at_rh(
//             vec3(2.0, 2.0, 2.0),
//             vec3(0.0, 0.0, 0.0),
//             vec3(0.0, 0.0, 1.0)
//         );

//         let mut proj = glam::Mat4::perspective_rh(
//             45.0 * DEG_TO_RAD,
//             self.data.swapchain_extent.width as f32 / self.data.swapchain_extent.height as f32,
//             0.1,
//             10.0,
//         );

//         proj.col_mut(1).y *= -1.0;

//         let ubo = UniformBufferObject { model, view, proj };

//         // Copy into persistently-mapped memory (faster than map/unmap each frame)
//         let mapped = self.data.uniform_buffers_mapped[image_index];
//         memcpy(&ubo, mapped.cast(), 1);

//         Ok(())
//     }

//     /// Recreates the swapchain for the Vulkan app
//     #[rustfmt::skip]
//     unsafe fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
//         let size = window.inner_size();
//         if size.width == 0 || size.height == 0 {
//             return Ok(());
//         }

//         self.device.device_wait_idle()?;
//         self.destroy_swapchain();
//         create_swapchain(window, &self.instance, &self.device, &mut self.data)?;
//         create_swapchain_image_views(&self.device, &mut self.data)?;
//         create_color_objects(&self.instance, &self.device, &mut self.data)?;
//         create_depth_objects(&self.instance, &self.device, &mut self.data)?;
//         create_render_pass(&self.instance, &self.device, &mut self.data)?;
//         create_pipeline(&self.instance, &self.device, &mut self.data)?;
//         create_framebuffers(&self.device, &mut self.data)?;
//         create_uniform_buffers(&self.instance, &self.device, &mut self.data)?;
//         create_descriptor_pool(&self.device, &mut self.data)?;
//         create_descriptor_sets(&self.device, &mut self.data)?;
//         create_command_buffers(&self.device, &mut self.data)?;
//         self.data.images_in_flight.resize(self.data.swapchain_images.len(), vk::Fence::null());
//         Ok(())
//     }

//     /// Destroys the Vulkan app
//     unsafe fn destroy(&mut self) {
//         // Ensure all submitted work is complete before destroying GPU resources.
        
//         self.device.device_wait_idle().unwrap();

//         // Destroy swapchain

//         self.destroy_swapchain();

//         // Destroy syncs

//         self.data.in_flight_fences.iter().for_each(|f| self.device.destroy_fence(*f, None));
//         self.data.render_finished_semaphores.iter().for_each(|f| self.device.destroy_semaphore(*f, None));
//         self.data.image_available_semaphores.iter().for_each(|f| self.device.destroy_semaphore(*f, None));

//         // Destroy Buffers

//         self.device.destroy_buffer(self.data.index_buffer, None);
//         self.device.free_memory(self.data.index_buffer_memory, None);
//         self.device.destroy_buffer(self.data.vertex_buffer, None);
//         self.device.free_memory(self.data.vertex_buffer_memory, None);

//         // Destroy textures
//         self.device.destroy_sampler(self.data.texture_sampler, None);
//         self.device.destroy_image_view(self.data.texture_image_view, None);
//         self.device.destroy_image(self.data.texture_image, None);
//         self.device.free_memory(self.data.texture_image_memory, None);

//         // Destroy command pool
        
//         self.device.destroy_command_pool(self.data.command_pool, None);
        
//         // Destroy descriptors

//         self.device.destroy_descriptor_set_layout(self.data.descriptor_set_layout, None);

//         // Destroy remaining
        
//         self.device.destroy_device(None);

//         if VALIDATION_ENABLED {
//             self.instance.destroy_debug_utils_messenger_ext(self.data.messenger, None);
//         }

//         self.instance.destroy_surface_khr(self.data.surface, None);
//         self.instance.destroy_instance(None);
//     }

//     /// Destroys the parts of our Vulkan app related to the swapchain
//     #[rustfmt::skip]
//     unsafe fn destroy_swapchain(&mut self) {
//         self.device.free_command_buffers(self.data.command_pool, &self.data.command_buffers);
//         self.device.destroy_descriptor_pool(self.data.descriptor_pool, None);
//         self.data.uniform_buffers.iter().for_each(|b| self.device.destroy_buffer(*b, None));
//         // Unmap persistent mappings and free memory
//         for &mem in &self.data.uniform_buffers_memory {
//             if !mem.is_null() {
//                 self.device.unmap_memory(mem);
//             }
//             self.device.free_memory(mem, None);
//         }
//         self.device.destroy_image_view(self.data.depth_image_view, None);
//         self.device.destroy_image(self.data.depth_image, None);
//         self.device.free_memory(self.data.depth_image_memory, None);
//         self.device.destroy_image_view(self.data.color_image_view, None);
//         self.device.destroy_image(self.data.color_image, None);
//         self.device.free_memory(self.data.color_image_memory, None);
//         self.data.framebuffers.iter().for_each(|f| self.device.destroy_framebuffer(*f, None));
//         self.device.destroy_pipeline(self.data.pipeline, None);
//         self.device.destroy_pipeline_layout(self.data.pipeline_layout, None);
//         self.device.destroy_render_pass(self.data.render_pass, None);
//         self.data.swapchain_image_views.iter().for_each(|v| self.device.destroy_image_view(*v, None));
//         self.device.destroy_swapchain_khr(self.data.swapchain, None);
//     }
// }

// /// The Vulkan handles and associated properties used by the Vulkan app.
// #[derive(Clone, Debug, Default)]
// struct AppData {
//     // Debug
//     messenger: vk::DebugUtilsMessengerEXT,
//     // Surface
//     surface: vk::SurfaceKHR,
//     // Devices
//     physical_device: vk::PhysicalDevice,
//     msaa_samples: vk::SampleCountFlags,
//     graphics_queue: vk::Queue,
//     present_queue: vk::Queue,
//     // Swapchain
//     swapchain_format: vk::Format,
//     swapchain_extent: vk::Extent2D,
//     swapchain: vk::SwapchainKHR,
//     swapchain_images: Vec<vk::Image>,
//     swapchain_image_views: Vec<vk::ImageView>,
//     uniform_buffers_mapped: Vec<*mut c_void>,
//     // Pipeline
//     render_pass: vk::RenderPass,
//     descriptor_set_layout: vk::DescriptorSetLayout,
//     pipeline_layout: vk::PipelineLayout,
//     pipeline: vk::Pipeline,
//     // Framebuffer
//     framebuffers: Vec<vk::Framebuffer>,
//     // Command Pool
//     command_pool: vk::CommandPool,
//     // Color
//     color_image: vk::Image,
//     color_image_memory: vk::DeviceMemory,
//     color_image_view: vk::ImageView,
//     // Depth
//     depth_image: vk::Image,
//     depth_image_memory: vk::DeviceMemory,
//     depth_image_view: vk::ImageView,
//     // Texture
//     texture_image: vk::Image,
//     texture_image_memory: vk::DeviceMemory,
//     texture_image_view: vk::ImageView,
//     texture_sampler: vk::Sampler,
//     // Model
//     vertices: Vec<QuantizedVertex>,
//     indices: Vec<u32>,
//     // Buffers
//     vertex_buffer: vk::Buffer,
//     vertex_buffer_memory: vk::DeviceMemory,
//     index_buffer: vk::Buffer,
//     index_buffer_memory: vk::DeviceMemory,
//     uniform_buffers: Vec<vk::Buffer>,
//     uniform_buffers_memory: Vec<vk::DeviceMemory>,
//     // Descriptors
//     descriptor_pool: vk::DescriptorPool,
//     descriptor_sets: Vec<vk::DescriptorSet>,
//     // Command Buffers
//     command_buffers: Vec<vk::CommandBuffer>,
//     // Sync Objects
//     image_available_semaphores: Vec<vk::Semaphore>,
//     render_finished_semaphores: Vec<vk::Semaphore>,
//     in_flight_fences: Vec<vk::Fence>,
//     images_in_flight: Vec<vk::Fence>,
// }

// //================================================
// // Instance
// //================================================

// unsafe fn create_instance(window: &Window, entry: &Entry, data: &mut AppData) -> Result<Instance> {
//     // Application Info
    
//     let application_info = vk::ApplicationInfo::builder()
//         .application_name(b"Vulkan Tutorial\0")
//         .application_version(vk::make_version(1, 0, 0))
//         .engine_name(b"No Engine\0")
//         .engine_version(vk::make_version(1, 0, 0))
//         .api_version(vk::make_version(1, 1, 0));

//     // Layers

//     let available_layers = entry
//         .enumerate_instance_layer_properties()?
//         .iter()
//         .map(|l| l.layer_name)
//         .collect::<HashSet<_>>();

//     if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
//         return Err(anyhow!("Validation layer requested but not supported"));
//     }

//     let layers = if VALIDATION_ENABLED {
//         vec![VALIDATION_LAYER.as_ptr()]
//     } else {
//         Vec::new()
//     };

//     // Extensions

//     // Get global required extensions for Vulkan to run
//     let mut extensions = vk_window::get_required_instance_extensions(window)
//         .iter()
//         .map(|e| e.as_ptr())
//         .collect::<Vec<_>>();


//     // Add macOS required extensions if user is on macOS
//     let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
//         info!("Enabling extensions for macOS portability");
//         extensions.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION.name.as_ptr());
//         extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
//         vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
//     } else {
//         vk::InstanceCreateFlags::empty()
//     };

//     if VALIDATION_ENABLED {
//         extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
//     }

//     // Create

//     let mut info = vk::InstanceCreateInfo::builder()
//         .application_info(&application_info)
//         .enabled_layer_names(&layers)
//         .enabled_extension_names(&extensions)
//         .flags(flags);

//     let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
//         .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
//         .message_type(
//             vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
//             | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
//             | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
//         )
//         .user_callback(Some(debug_callback));
    
//     if VALIDATION_ENABLED {
//         info = info.push_next(&mut debug_info);
//     }

//     let instance = entry.create_instance(&info, None)?;

//     // Messenger

//     if VALIDATION_ENABLED {
//         data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
//     }

//     Ok(instance)
// }

// extern "system" fn debug_callback(
//     severity: vk::DebugUtilsMessageSeverityFlagsEXT,
//     type_: vk::DebugUtilsMessageTypeFlagsEXT,
//     data: *const vk::DebugUtilsMessengerCallbackDataEXT,
//     _: *mut c_void,
// ) -> vk::Bool32 {
//     let data = unsafe { *data };
//     let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

//     if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
//         error!("({:?}) {}", type_, message);
//     } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
//         warn!("({:?}) {}", type_, message);
//     } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
//         debug!("({:?}) {}", type_, message);
//     } else {
//         trace!("({:?}) {}", type_, message);
//     }

//     vk::FALSE
// }

// //================================================
// // Physical Device
// //================================================

// #[derive(Debug, Error)]
// #[error("{0}")]
// pub struct SuitabilityError(pub &'static str);

// unsafe fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()> {

//     let chosen_physical_device = Some(*instance.enumerate_physical_devices()?
//         .iter()
//         .filter_map(|p| {
//             let properties = instance.get_physical_device_properties(*p);
//             if let Err(error) = check_physical_device(instance, data, *p) {
//                 warn!("Skipping physical device ('{}'): {}", properties.device_name, error);
//                 None
//             } else {
//                 info!("Found available physical device ('{}')\n\tDevice type ('{:?}')\n\tPush constant size ({})\n\tMax image dimension 2d ({})",
//                 properties.device_name,
//                 properties.device_type,
//                 properties.limits.max_push_constants_size,
//                 properties.limits.max_image_dimension_2d,
//             );
//                 Some(p)
//             }
//         })
//         .max_by_key(|p| {
//             // lower score for preferred device types
//             let properties = instance.get_physical_device_properties(**p);

//             match properties.device_type {
//                 vk::PhysicalDeviceType::DISCRETE_GPU => 10000 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                 vk::PhysicalDeviceType::INTEGRATED_GPU => 1000 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                 vk::PhysicalDeviceType::VIRTUAL_GPU => 100 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                 vk::PhysicalDeviceType::CPU => 10 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                 vk::PhysicalDeviceType::OTHER => 1 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                 _ => 0
//             }

//         })
//         .unwrap());

//     if chosen_physical_device != None {
//         info!("Chose physical device ('{}')", instance.get_physical_device_properties(chosen_physical_device.unwrap()).device_name);
//         data.physical_device = chosen_physical_device.unwrap();
//         let max_msaa = get_max_msaa_samples(instance, data);
//         let chosen_msaa = if max_msaa < vk::SampleCountFlags::_8 { max_msaa } else { vk::SampleCountFlags::_8 }; 
//         info!("Max msaa detected: {:?}", max_msaa);
//         info!("Chosen msaa: {:?}", chosen_msaa);
//         data.msaa_samples = chosen_msaa;
//         return Ok(());
//     }

//     Err(anyhow!("Failed to find suitable physical device"))
// }

// unsafe fn check_physical_device(
//     instance: &Instance,
//     data: &mut AppData,
//     physical_device: vk::PhysicalDevice,
// ) -> Result<()> {
//     QueueFamilyIndices::get(instance, data, physical_device)?;
//     check_physical_device_extensions(instance, physical_device)?;

//     let support = SwapchainSupport::get(instance, data, physical_device)?;
//     if support.formats.is_empty() || support.present_modes.is_empty() {
//         return Err(anyhow!(SuitabilityError("Insufficient swapchain support")))
//     }

//     let features = instance.get_physical_device_features(physical_device);
//     if features.sampler_anisotropy != vk::TRUE {
//         return Err(anyhow!(SuitabilityError("No sampler anisotropy")));
//     }

//     Ok(())
// }

// unsafe fn check_physical_device_extensions(instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<()> {
//     let extensions = instance
//         .enumerate_device_extension_properties(physical_device, None)?
//         .iter()
//         .map(|e| e.extension_name)
//         .collect::<HashSet<_>>();

//     if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
//         Ok(())
//     } else {
//         Err(anyhow!(SuitabilityError("Missing required device extensions")))
//     }
// }

// unsafe fn get_max_msaa_samples(instance: &Instance, data: &AppData) -> vk::SampleCountFlags {
//     let properties = instance.get_physical_device_properties(data.physical_device);
//     let counts = properties.limits.framebuffer_color_sample_counts & properties.limits.framebuffer_depth_sample_counts;
//     [
//         vk::SampleCountFlags::_64,
//         vk::SampleCountFlags::_32,
//         vk::SampleCountFlags::_16,
//         vk::SampleCountFlags::_8,
//         vk::SampleCountFlags::_4,
//         vk::SampleCountFlags::_2,
//     ]
//     .iter()
//     .cloned()
//     .find(|c| counts.contains(*c))
//     .unwrap_or(vk::SampleCountFlags::_1)
// }

// //================================================
// // Logical Device
// //================================================

// unsafe fn create_logical_device(entry: &Entry, instance: &Instance, data: &mut AppData) -> Result<Device> {
//     // Queue Create Infos

//     let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

//     let mut unique_indices = HashSet::new();
//     unique_indices.insert(indices.graphics);
//     unique_indices.insert(indices.present);

//     let queue_priorities = &[1.0];
//     let queue_infos = unique_indices
//         .iter()
//         .map(|i| {
//             vk::DeviceQueueCreateInfo::builder()
//             .queue_family_index(*i)
//             .queue_priorities(queue_priorities)
//         })
//         .collect::<Vec<_>>();
    
//     // Layers

//     let layers = if VALIDATION_ENABLED {
//         vec![VALIDATION_LAYER.as_ptr()]
//     } else {
//         vec![]
//     };

//     // Extensions

//     let mut extensions = DEVICE_EXTENSIONS
//         .iter()
//         .map(|n| n.as_ptr())
//         .collect::<Vec<_>>();

//     // Required by Vulkan SDK on macOS
//     if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
//         extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
//     }

//     // Enforce shader draw parameters for slang shaders
//     //extensions.push(vk::KHR_SHADER_DRAW_PARAMETERS_EXTENSION.name.as_ptr());

//     // Features

//     let features = vk::PhysicalDeviceFeatures::builder()
//         .sampler_anisotropy(true)
//         .sample_rate_shading(true);

//     // Create

//     let info = vk::DeviceCreateInfo::builder()
//         .queue_create_infos(&queue_infos)
//         .enabled_layer_names(&layers)
//         .enabled_extension_names(&extensions)
//         .enabled_features(&features);

//     let device = instance.create_device(data.physical_device, &info, None)?;

//     // Queues

//     data.graphics_queue = device.get_device_queue(indices.graphics, 0);
//     data.present_queue = device.get_device_queue(indices.present, 0);

//     Ok(device)
// }

// //================================================
// // Swapchain
// //================================================

// unsafe fn create_swapchain(
//     window: &Window,
//     instance: &Instance,
//     device: &Device,
//     data: &mut AppData,
// ) -> Result<()> {
//     // Image

//     let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
//     let support = SwapchainSupport::get(instance, data, data.physical_device)?;

//     let surface_format = get_swapchain_surface_format(&support.formats);
//     let present_mode = get_swapchain_present_mode(&support.present_modes);
//     let extent = get_swapchain_extent(window, support.capabilities);

//     data.swapchain_format = surface_format.format;
//     data.swapchain_extent = extent;

//     let mut image_count = support.capabilities.min_image_count + 1;
//     if support.capabilities.max_image_count != 0 && image_count > support.capabilities.max_image_count {
//         image_count = support.capabilities.max_image_count
//     }

//     let mut queue_family_indices = vec![];
//     let image_sharing_mode = if indices.graphics != indices.present {
//         queue_family_indices.push(indices.graphics);
//         queue_family_indices.push(indices.present);
//         vk::SharingMode::CONCURRENT
//     } else {
//         vk::SharingMode::EXCLUSIVE
//     };

//     // Create

//     let info = vk::SwapchainCreateInfoKHR::builder()
//         .surface(data.surface)
//         .min_image_count(image_count)
//         .image_format(surface_format.format)
//         .image_color_space(surface_format.color_space)
//         .image_extent(extent)
//         .image_array_layers(1)
//         .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
//         .image_sharing_mode(image_sharing_mode)
//         .queue_family_indices(&queue_family_indices)
//         .pre_transform(support.capabilities.current_transform)
//         .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
//         .present_mode(present_mode)
//         .clipped(true)
//         .old_swapchain(vk::SwapchainKHR::null());

//     data.swapchain = device.create_swapchain_khr(&info, None)?;

//     // Images

//     data.swapchain_images = device.get_swapchain_images_khr(data.swapchain)?;

//     Ok(())
// }

// // fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
// //     formats
// //         .iter()
// //         .cloned()
// //         .find(|f| f.format == vk::Format::B8G8R8_SRGB && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
// //         .unwrap_or_else(|| formats[0])
// // }

// fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
//     let format = formats
//         .iter()
//         .cloned()
//         .find(|f| (f.format == vk::Format::B8G8R8_SRGB || f.format == vk::Format::R8G8B8_SRGB) 
//             && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
//         .or_else(|| formats
//             .iter()
//             .cloned()
//             .find(|f| f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR))
//         .unwrap_or_else(|| formats[0]);
    
//     info!("Selected swapchain format: {:?}, color space: {:?}", format.format, format.color_space);
//     format
// }

// fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
//     present_modes
//         .iter()
//         .cloned()
//     .find(|m| *m == vk::PresentModeKHR::IMMEDIATE)
//     .or_else(|| present_modes.iter().cloned().find(|m| *m == vk::PresentModeKHR::MAILBOX))
//     .unwrap_or(vk::PresentModeKHR::FIFO)
// }

// #[rustfmt::skip]
// fn get_swapchain_extent(window: &Window, capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
//     if capabilities.current_extent.width != u32::MAX {
//         capabilities.current_extent
//     } else {
//         vk::Extent2D::builder()
//             .width(window.inner_size().width.clamp(
//                 capabilities.min_image_extent.width,
//                 capabilities.max_image_extent.width,
//             ))
//             .height(window.inner_size().height.clamp(
//                 capabilities.min_image_extent.height,
//                 capabilities.max_image_extent.height,
//             ))
//             .build()
//     }
// }

// unsafe fn create_swapchain_image_views(device: &Device, data: &mut AppData) -> Result<()> {
//     data.swapchain_image_views = data
//         .swapchain_images
//         .iter()
//         .map(|i| create_image_view(device, *i, data.swapchain_format, vk::ImageAspectFlags::COLOR, 1))
//         .collect::<Result<Vec<_>, _>>()?;

//     Ok(())
// }

// //================================================
// // Pipeline
// //================================================

// unsafe fn create_render_pass(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Attachments

//     let color_attachment = vk::AttachmentDescription::builder()
//         .format(data.swapchain_format)
//         .samples(data.msaa_samples)
//         .load_op(vk::AttachmentLoadOp::CLEAR)
//         .store_op(vk::AttachmentStoreOp::STORE)
//         .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
//         .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
//         .initial_layout(vk::ImageLayout::UNDEFINED)
//         .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

//     let depth_stencil_attachment = vk::AttachmentDescription::builder()
//         .format(get_depth_format(instance, data)?)
//         .samples(data.msaa_samples)
//         .load_op(vk::AttachmentLoadOp::CLEAR)
//         .store_op(vk::AttachmentStoreOp::DONT_CARE)
//         .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
//         .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
//         .initial_layout(vk::ImageLayout::UNDEFINED)
//         .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

//     let color_resolve_attachment = vk::AttachmentDescription::builder()
//         .format(data.swapchain_format)
//         .samples(vk::SampleCountFlags::_1)
//         .load_op(vk::AttachmentLoadOp::DONT_CARE)
//         .store_op(vk::AttachmentStoreOp::STORE)
//         .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
//         .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
//         .initial_layout(vk::ImageLayout::UNDEFINED)
//         .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

//     // Subpasses

//     let color_attachment_ref = vk::AttachmentReference::builder()
//         .attachment(0)
//         .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

//     let depth_stencil_attachment_ref = vk::AttachmentReference::builder()
//         .attachment(1)
//         .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

//     let color_resolve_attachment_ref = vk::AttachmentReference::builder()
//         .attachment(2)
//         .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

//     let color_attachments = &[color_attachment_ref];
//     let resolve_attachments = &[color_resolve_attachment_ref];
//     let subpass = vk::SubpassDescription::builder()
//         .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
//         .color_attachments(color_attachments)
//         .depth_stencil_attachment(&depth_stencil_attachment_ref)
//         .resolve_attachments(resolve_attachments);

//     // Dependencies

//     let dependency = vk::SubpassDependency::builder()
//         .src_subpass(vk::SUBPASS_EXTERNAL)
//         .dst_subpass(0)
//         .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
//         .src_access_mask(vk::AccessFlags::empty())
//         .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS)
//         .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);

//     // Create

//     let attachments = &[color_attachment, depth_stencil_attachment, color_resolve_attachment];
//     let subpasses = &[subpass];
//     let dependencies = &[dependency];
//     let info = vk::RenderPassCreateInfo::builder()
//         .attachments(attachments)
//         .subpasses(subpasses)
//         .dependencies(dependencies);

//     data.render_pass = device.create_render_pass(&info, None)?;

//     Ok(())
// }

// unsafe fn create_descriptor_set_layout(device: &Device, data: &mut AppData) -> Result<()> {
//     let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
//         .binding(0)
//         .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
//         .descriptor_count(1)
//         .stage_flags(vk::ShaderStageFlags::VERTEX);

//     let sampler_binding = vk::DescriptorSetLayoutBinding::builder()
//         .binding(1)
//         .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
//         .descriptor_count(1)
//         .stage_flags(vk::ShaderStageFlags::FRAGMENT);

//     let bindings = &[ubo_binding, sampler_binding];
//     let info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings);

//     data.descriptor_set_layout = device.create_descriptor_set_layout(&info, None)?;

//     Ok(())
// }

// unsafe fn create_pipeline(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Stages

//     let shader = include_bytes!("../assets/shaders/shader.spv");
    
//     let shader_module = create_shader_module(device, &shader[..])?;

//     let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
//         .stage(vk::ShaderStageFlags::VERTEX)
//         .module(shader_module)
//         .name(b"vertMain\0");

//     let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
//         .stage(vk::ShaderStageFlags::FRAGMENT)
//         .module(shader_module)
//         .name(b"fragMain\0");

//     // Vertex Input State

//     let binding_descriptions = &[QuantizedVertex::binding_description()];
//     let attribute_descriptions = QuantizedVertex::attribute_descriptions_with_fallback(instance, data.physical_device)?;
//     let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
//         .vertex_binding_descriptions(binding_descriptions)
//         .vertex_attribute_descriptions(&attribute_descriptions);

//     // Input Assembly State

//     let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
//         .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
//         .primitive_restart_enable(false);

//     // Viewport State

//     let viewport = vk::Viewport::builder()
//         .x(0.0)
//         .y(0.0)
//         .width(data.swapchain_extent.width as f32)
//         .height(data.swapchain_extent.height as f32)
//         .min_depth(0.0)
//         .max_depth(1.0);

//     let scissor = vk::Rect2D::builder()
//         .offset(vk::Offset2D { x: 0, y: 0 })
//         .extent(data.swapchain_extent);

//     let viewports = &[viewport];
//     let scissors = &[scissor];
//     let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
//         .viewports(viewports)
//         .scissors(scissors);

//     // Rasterization State

//     let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
//         .depth_bias_enable(false)
//         .rasterizer_discard_enable(false)
//         .polygon_mode(vk::PolygonMode::FILL)
//         .line_width(1.0)
//         .cull_mode(vk::CullModeFlags::BACK)
//         .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
//         .depth_bias_enable(false);

//     // Multisample State

//     let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
//         .sample_shading_enable(true)
//         .min_sample_shading(0.2)
//         .rasterization_samples(data.msaa_samples);

//     let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
//         .depth_test_enable(true)
//         .depth_write_enable(true)
//         .depth_compare_op(vk::CompareOp::LESS)
//         .depth_bounds_test_enable(false)
//         .stencil_test_enable(false);

//     // Color Blend State

//     let attachment = vk::PipelineColorBlendAttachmentState::builder()
//         .color_write_mask(vk::ColorComponentFlags::all())
//         .blend_enable(false);

//     let attachments = &[attachment];
//     let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
//         .logic_op_enable(false)
//         .logic_op(vk::LogicOp::COPY)
//         .attachments(attachments)
//         .blend_constants([0.0, 0.0, 0.0, 0.0]);

//     // Layout

//     let set_layouts = &[data.descriptor_set_layout];
//     let layout_info = vk::PipelineLayoutCreateInfo::builder()
//         .set_layouts(set_layouts);
//     data.pipeline_layout = device.create_pipeline_layout(&layout_info, None)?;

//     // Create

//     let stages = &[vert_stage, frag_stage];
//     let info = vk::GraphicsPipelineCreateInfo::builder()
//         .stages(stages)
//         .vertex_input_state(&vertex_input_state)
//         .input_assembly_state(&input_assembly_state)
//         .viewport_state(&viewport_state)
//         .rasterization_state(&rasterization_state)
//         .multisample_state(&multisample_state)
//         .depth_stencil_state(&depth_stencil_state)
//         .color_blend_state(&color_blend_state)
//         .layout(data.pipeline_layout)
//         .render_pass(data.render_pass)
//         .subpass(0);

//     data.pipeline = device
//         .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
//         .0[0];

//     // Cleanup

//     device.destroy_shader_module(shader_module, None);

//     Ok(())
// }

// unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
//     let bytecode = Bytecode::new(bytecode).unwrap();
//     let info = vk::ShaderModuleCreateInfo::builder()
//         .code(bytecode.code())
//         .code_size(bytecode.code_size());
//     Ok(device.create_shader_module(&info, None)?)
// }

// //================================================
// // Framebuffers
// //================================================

// unsafe fn create_framebuffers(device: &Device, data: &mut AppData) -> Result<()> {
//     data.framebuffers = data
//         .swapchain_image_views
//         .iter()
//         .map(|i| {
//             let attachments = &[data.color_image_view, data.depth_image_view, *i];
//             let create_info = vk::FramebufferCreateInfo::builder()
//                 .render_pass(data.render_pass)
//                 .attachments(attachments)
//                 .width(data.swapchain_extent.width)
//                 .height(data.swapchain_extent.height)
//                 .layers(1);

//             device.create_framebuffer(&create_info, None)
//         })
//         .collect::<Result<Vec<_>, _>>()?;

//     Ok(())
// }

// //================================================
// // Command Pool
// //================================================

// unsafe fn create_command_pool(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;

//     let info = vk::CommandPoolCreateInfo::builder().queue_family_index(indices.graphics);

//     data.command_pool = device.create_command_pool(&info, None)?;

//     Ok(())
// }

// //================================================
// // Color Objects
// //================================================

// unsafe fn create_color_objects(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Image + Image Memory

//     let (color_image, color_image_memory) = create_image(
//         instance,
//         device,
//         data,
//         data.swapchain_extent.width,
//         data.swapchain_extent.height,
//         1,
//         data.msaa_samples,
//         data.swapchain_format,
//         vk::ImageTiling::OPTIMAL,
//         vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
//         vk::MemoryPropertyFlags::DEVICE_LOCAL
//     )?;

//     data.color_image = color_image;
//     data.color_image_memory = color_image_memory;

//     // Image View

//     data.color_image_view = create_image_view(
//         device,
//         data.color_image,
//         data.swapchain_format,
//         vk::ImageAspectFlags::COLOR,
//         1,
//     )?;

//     Ok(())
// }

// //================================================
// // Depth Objects
// //================================================

// unsafe fn create_depth_objects(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Image + Image Memory

//     let format = get_depth_format(instance, data)?;

//     let (depth_image, depth_image_memory) = create_image(
//         instance,
//         device,
//         data,
//         data.swapchain_extent.width,
//         data.swapchain_extent.height,
//         1,
//         data.msaa_samples,
//         format,
//         vk::ImageTiling::OPTIMAL,
//         vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
//         vk::MemoryPropertyFlags::DEVICE_LOCAL,
//     )?;

//     data.depth_image = depth_image;
//     data.depth_image_memory = depth_image_memory;

//     // Image view

//     data.depth_image_view = create_image_view(device, data.depth_image, format, vk::ImageAspectFlags::DEPTH, 1)?;

//     Ok(())
// }

// unsafe fn get_depth_format(instance: &Instance, data: &AppData) -> Result<vk::Format> {
//     let candidates = &[
//         vk::Format::D32_SFLOAT,
//         vk::Format::D32_SFLOAT_S8_UINT,
//         vk::Format::D24_UNORM_S8_UINT,
//     ];

//     get_supported_format(
//         instance,
//         data,
//         candidates,
//         vk::ImageTiling::OPTIMAL,
//         vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
//     )
// }

// unsafe fn get_supported_format(
//     instance: &Instance,
//     data: &AppData,
//     candidates: &[vk::Format],
//     tiling: vk::ImageTiling,
//     features: vk::FormatFeatureFlags,
// ) -> Result<vk::Format> {
//     candidates
//         .iter()
//         .cloned()
//         .find(|f| {
//             let properties = instance.get_physical_device_format_properties(data.physical_device, *f);
//             match tiling {
//                 vk::ImageTiling::LINEAR => properties.linear_tiling_features.contains(features),
//                 vk::ImageTiling::OPTIMAL => properties.optimal_tiling_features.contains(features),
//                 _ => false,
//             }
//         })
//         .ok_or_else(|| anyhow!("Failed to find supported format"))
// }

// fn get_supported_vertex_format(
//     instance: &Instance,
//     physical_device: vk::PhysicalDevice,
//     candidates: &[vk::Format],
//     features: vk::FormatFeatureFlags,
// ) -> Result<vk::Format> {
//     candidates
//         .iter()
//         .cloned()
//         .find(|f| {
//             let properties = unsafe { instance.get_physical_device_format_properties(physical_device, *f) };
//             // For vertex buffers, check buffer features (typically linear tiling)
//             properties.buffer_features.contains(features)
//         })
//         .ok_or_else(|| anyhow!("Failed to find supported vertex attribute format"))
// }

// fn is_texture_format_supported(
//     instance: &Instance,
//     physical_device: vk::PhysicalDevice,
//     format: vk::Format,
// ) -> bool {
//     let properties = unsafe { instance.get_physical_device_format_properties(physical_device, format) };
//     // Check if format is supported for optimal tiling with sampled image feature
//     properties.optimal_tiling_features.contains(vk::FormatFeatureFlags::SAMPLED_IMAGE)
// }

// //================================================
// // Texture
// //================================================

// unsafe fn create_texture_image(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Load

//     let texture = {
//         let image = include_bytes!("../assets/textures/cuttlefish_albedo.ktx2");
//         let mut texture = ktx2_rw::Ktx2Texture::from_memory(image)?;
        
//         // Try BC7 first, fall back to ASTC 4x4 if not supported
//         let transcode_format = if is_texture_format_supported(instance, data.physical_device, vk::Format::BC7_SRGB_BLOCK) {
//             info!("Using BC7 format for texture transcoding");
//             ktx2_rw::TranscodeFormat::Bc7Rgba
//         } else if is_texture_format_supported(instance, data.physical_device, vk::Format::ASTC_4X4_SRGB_BLOCK) {
//             info!("BC7 not supported, falling back to ASTC 4x4 for texture transcoding");
//             ktx2_rw::TranscodeFormat::Astc_4x4_Rgba
//         } else {
//             return Err(anyhow!("Neither BC7 nor ASTC 4x4 compression formats are supported"));
//         };
        
//         texture.transcode_basis(transcode_format).expect("Failed to transcode texture image format");
//         texture
//     };

//     let format = vk::Format::from_raw(texture.vk_format().as_raw() as i32);
//     let pixel_data = texture.get_image_data(0, 0, 0).unwrap();
//     let mipmap_levels = texture.levels();

//     // Calculate total size for all mip levels and collect per-level data
//     let mut mip_sizes: Vec<usize> = Vec::with_capacity(mipmap_levels as usize);
//     let mut total_size: u64 = 0;
//     for level in 0..mipmap_levels {
//         let mip_pixel_data = texture.get_image_data(level, 0, 0).unwrap();
//         mip_sizes.push(mip_pixel_data.len());
//         total_size += mip_pixel_data.len() as u64;
//     }

//     // Create (staging)

//     let (staging_buffer, staging_buffer_memory) = create_buffer(
//         instance,
//         device,
//         data,
//         total_size,
//         vk::BufferUsageFlags::TRANSFER_SRC,
//         vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
//     )?;

//     // Copy (staging)

//     let memory = device.map_memory(staging_buffer_memory, 0, total_size, vk::MemoryMapFlags::empty())?;

//     // Copy each mip level into the staging buffer at the correct offset
//     let mut offset: usize = 0;
//     for level in 0..mipmap_levels as usize {
//         let mip_pixel_data = texture.get_image_data(level as u32, 0, 0).unwrap();
//         memcpy(mip_pixel_data.as_ptr(), memory.add(offset).cast(), mip_pixel_data.len());
//         offset += mip_pixel_data.len();
//     }

//     device.unmap_memory(staging_buffer_memory);

//     // Create (Image)

//     let (texture_image, texture_image_memory) = create_image(
//         instance,
//         device,
//         data,
//         texture.width(),
//         texture.height(),
//         mipmap_levels,
//         vk::SampleCountFlags::_1,
//         format,
//         vk::ImageTiling::OPTIMAL,
//         vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
//         vk::MemoryPropertyFlags::DEVICE_LOCAL,
//     )?;

//     data.texture_image = texture_image;
//     data.texture_image_memory = texture_image_memory;

//     // Transition + Copy (image)

//     transition_image_layout(
//         device,
//         data,
//         data.texture_image,
//         format,
//         vk::ImageLayout::UNDEFINED,
//         vk::ImageLayout::TRANSFER_DST_OPTIMAL,
//         mipmap_levels,
//     )?;
//     // Copy each mip level from the staging buffer into the corresponding image mip level
//     let command_buffer = begin_single_time_commands(device, data)?;

//     let mut buffer_offset: u64 = 0;
//     let mut regions: Vec<vk::BufferImageCopy> = Vec::with_capacity(mipmap_levels as usize);
//     for level in 0..mipmap_levels {
//         let mip_width = (texture.width() >> level).max(1);
//         let mip_height = (texture.height() >> level).max(1);

//         let subresource = vk::ImageSubresourceLayers::builder()
//             .aspect_mask(vk::ImageAspectFlags::COLOR)
//             .mip_level(level)
//             .base_array_layer(0)
//             .layer_count(1)
//             .build();

//         let region = vk::BufferImageCopy::builder()
//             .buffer_offset(buffer_offset)
//             .buffer_row_length(0)
//             .buffer_image_height(0)
//             .image_subresource(subresource)
//             .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
//             .image_extent(vk::Extent3D { width: mip_width, height: mip_height, depth: 1 })
//             .build();

//         regions.push(region);

//         buffer_offset += mip_sizes[level as usize] as u64;
//     }

//     device.cmd_copy_buffer_to_image(
//         command_buffer,
//         staging_buffer,
//         data.texture_image,
//         vk::ImageLayout::TRANSFER_DST_OPTIMAL,
//         &regions,
//     );

//     end_single_time_commands(device, data, command_buffer)?;

//     transition_image_layout(
//         device,
//         data,
//         data.texture_image,
//         format,
//         vk::ImageLayout::TRANSFER_DST_OPTIMAL,
//         vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
//         mipmap_levels,
//     )?;

//     // Cleanup

//     device.destroy_buffer(staging_buffer, None);
//     device.free_memory(staging_buffer_memory, None);

//     create_texture_image_view(&device, data, format, mipmap_levels)?;

//     create_texture_sampler(&device, data, mipmap_levels)?;

//     Ok(())
// }

// unsafe fn create_texture_image_view(device: &Device, data: &mut AppData, format: vk::Format, mipmap_levels: u32) -> Result<()> {
//     data.texture_image_view = create_image_view(device, data.texture_image, format, vk::ImageAspectFlags::COLOR, mipmap_levels)?;

//     Ok(())
// }

// unsafe fn create_texture_sampler(device: &Device, data: &mut AppData, mipmap_levels: u32) -> Result<()> {
//     let info = vk::SamplerCreateInfo::builder()
//         .mag_filter(vk::Filter::LINEAR)
//         .min_filter(vk::Filter::LINEAR)
//         .address_mode_u(vk::SamplerAddressMode::REPEAT)
//         .address_mode_v(vk::SamplerAddressMode::REPEAT)
//         .address_mode_w(vk::SamplerAddressMode::REPEAT)
//         .anisotropy_enable(true)
//         .max_anisotropy(16.0)
//         .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
//         .unnormalized_coordinates(false)
//         .compare_enable(false)
//         .compare_op(vk::CompareOp::ALWAYS)
//         .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
//         .min_lod(0.0)
//         .max_lod(mipmap_levels as f32)
//         .mip_lod_bias(0.0);

//     data.texture_sampler = device.create_sampler(&info, None)?;

//     Ok(())
// }

// //================================================
// // Model
// //================================================

// /// TODO: Consider rare and experimental BCn texture encoding techniques on vertex data to reduce vram usage and improve performance
// fn load_model(data: &mut AppData) -> Result<(Vec<QuantizedVertex>, Vec<u32>)> {
//     // Get vertices

//     let vertex_bytes: &[u8; _] = include_bytes!("../assets/models_compressed/Limpet.vertbuff");
//     let vertex_count = u32::from_be_bytes(vertex_bytes[0..size_of::<u32>()].try_into().unwrap()) as usize;

//     let quantized_vertices: Vec<QuantizedVertex> = match meshopt::decode_vertex_buffer(vertex_bytes[size_of::<u32>()..].try_into().unwrap(), vertex_count) {
//         Ok(bytes) => bytes,
//         Err(_) => return Err(anyhow!("Failed to decode vertex buffer")),
//     };

//     // Get indices

//     let index_bytes: &[u8; _] = include_bytes!("../assets/models_compressed/Limpet.indbuff");
//     let index_count = u32::from_be_bytes(index_bytes[0..size_of::<u32>()].try_into().unwrap()) as usize;
//     let indices: Vec<u32> = match meshopt::decode_index_buffer(index_bytes[size_of::<u32>()..].try_into().unwrap(), index_count) {
//         Ok(indices) => indices,
//         Err(_) => return Err(anyhow!("Failed to decode index buffer")),
//     };

//     Ok((quantized_vertices, indices))
// }

// //================================================
// // Buffers
// //================================================

// unsafe fn create_vertex_buffer(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Create (staging)

//     let size = (size_of::<QuantizedVertex>() * data.vertices.len()) as u64;

//     let (staging_buffer, staging_buffer_memory) = create_buffer(
//         instance,
//         device,
//         data,
//         size,
//         vk::BufferUsageFlags::TRANSFER_SRC,
//         vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
//     )?;

//     // Copy (staging)

//     let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;

//     memcpy(data.vertices.as_ptr(), memory.cast(), data.vertices.len());

//     device.unmap_memory(staging_buffer_memory);

//     // Create (vertex)

//     let (vertex_buffer, vertex_buffer_memory) = create_buffer(
//         instance,
//         device,
//         data,
//         size,
//         vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
//         vk::MemoryPropertyFlags::DEVICE_LOCAL,
//     )?;

//     data.vertex_buffer = vertex_buffer;
//     data.vertex_buffer_memory = vertex_buffer_memory;

//     // Copy (vertex)

//     copy_buffer(device, data, staging_buffer, vertex_buffer, size)?;

//     // Cleanup

//     device.destroy_buffer(staging_buffer, None);
//     device.free_memory(staging_buffer_memory, None);

//     Ok(())
// }

// unsafe fn create_index_buffer(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     // Create (staging)

//     let size = (size_of::<u32>() * data.indices.len()) as u64;

//     let (staging_buffer, staging_buffer_memory) = create_buffer(
//         instance,
//         device,
//         data,
//         size,
//         vk::BufferUsageFlags::TRANSFER_SRC,
//         vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
//     )?;

//     // Copy (staging)

//     let memory = device.map_memory(staging_buffer_memory, 0, size, vk::MemoryMapFlags::empty())?;

//     memcpy(data.indices.as_ptr(), memory.cast(), data.indices.len());

//     device.unmap_memory(staging_buffer_memory);

//     // Create (index)

//     let (index_buffer, index_buffer_memory) = create_buffer(
//         instance,
//         device,
//         data,
//         size,
//         vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
//         vk::MemoryPropertyFlags::DEVICE_LOCAL,
//     )?;

//     data.index_buffer = index_buffer;
//     data.index_buffer_memory = index_buffer_memory;

//     // Copy (index)

//     copy_buffer(device, data, staging_buffer, index_buffer, size)?;

//     // Cleanup

//     device.destroy_buffer(staging_buffer, None);
//     device.free_memory(staging_buffer_memory, None);

//     Ok(())
// }

// unsafe fn create_uniform_buffers(instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> {
//     data.uniform_buffers.clear();
//     data.uniform_buffers_memory.clear();
//     data.uniform_buffers_mapped.clear();

//     for _ in 0..data.swapchain_images.len() {
//         let (uniform_buffer, uniform_buffer_memory) = create_buffer(
//             instance,
//             device,
//             data,
//             size_of::<UniformBufferObject>() as u64,
//             vk::BufferUsageFlags::UNIFORM_BUFFER,
//             vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
//         )?;

//         data.uniform_buffers.push(uniform_buffer);
//         data.uniform_buffers_memory.push(uniform_buffer_memory);
//         // Persistently map the uniform buffer memory to avoid map/unmap each frame
//         let mapped = device.map_memory(uniform_buffer_memory, 0, size_of::<UniformBufferObject>() as u64, vk::MemoryMapFlags::empty())?;
//         data.uniform_buffers_mapped.push(mapped.cast());
//     }

//     Ok(())
// }

// //================================================
// // Descriptors
// //================================================

// unsafe fn create_descriptor_pool(device: &Device, data: &mut AppData) -> Result<()> {
//     let ubo_size = vk::DescriptorPoolSize::builder()
//         .type_(vk::DescriptorType::UNIFORM_BUFFER)
//         .descriptor_count(data.swapchain_images.len() as u32);

//     let sampler_size = vk::DescriptorPoolSize::builder()
//         .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
//         .descriptor_count(data.swapchain_images.len() as u32);

//     let pool_sizes = &[ubo_size, sampler_size];
//     let info = vk::DescriptorPoolCreateInfo::builder()
//         .pool_sizes(pool_sizes)
//         .max_sets(data.swapchain_images.len() as u32);

//     data.descriptor_pool = device.create_descriptor_pool(&info, None)?;

//     Ok(())
// }

// unsafe fn create_descriptor_sets(device: &Device, data: &mut AppData) -> Result<()> {
//     // Allocate

//     let layouts = vec![data.descriptor_set_layout; data.swapchain_images.len()];
//     let info = vk::DescriptorSetAllocateInfo::builder()
//         .descriptor_pool(data.descriptor_pool)
//         .set_layouts(&layouts);

//     data.descriptor_sets = device.allocate_descriptor_sets(&info)?;

//     // Update

//     for i in 0..data.swapchain_images.len() {
//         let info = vk::DescriptorBufferInfo::builder()
//             .buffer(data.uniform_buffers[i])
//             .offset(0)
//             .range(size_of::<UniformBufferObject>() as u64);

//         let buffer_info = &[info];
//         let ubo_write = vk::WriteDescriptorSet::builder()
//             .dst_set(data.descriptor_sets[i])
//             .dst_binding(0)
//             .dst_array_element(0)
//             .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
//             .buffer_info(buffer_info);

//         let info = vk::DescriptorImageInfo::builder()
//             .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
//             .image_view(data.texture_image_view)
//             .sampler(data.texture_sampler);

//         let image_info = &[info];
//         let sampler_write = vk::WriteDescriptorSet::builder()
//             .dst_set(data.descriptor_sets[i])
//             .dst_binding(1)
//             .dst_array_element(0)
//             .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
//             .image_info(image_info);

//         device.update_descriptor_sets(&[ubo_write, sampler_write], &[] as &[vk::CopyDescriptorSet]);
//     }

//     Ok(())
// }

// //================================================
// // Command Buffers
// //================================================

// unsafe fn create_command_buffers(device: &Device, data: &mut AppData) -> Result<()> {
//     // Allocate

//     let allocate_info = vk::CommandBufferAllocateInfo::builder()
//         .command_pool(data.command_pool)
//         .level(vk::CommandBufferLevel::PRIMARY)
//         .command_buffer_count(data.framebuffers.len() as u32);

//     data.command_buffers = device.allocate_command_buffers(&allocate_info)?;

//     // Commands

//     for (i, command_buffer) in data.command_buffers.iter().enumerate() {
//         let info = vk::CommandBufferBeginInfo::builder();

//         device.begin_command_buffer(*command_buffer, &info)?;

//         let render_area = vk::Rect2D::builder()
//             .offset(vk::Offset2D::default())
//             .extent(data.swapchain_extent);

//         let color_clear_value = vk::ClearValue {
//             color: vk::ClearColorValue {
//                 float32: [0.0, 0.0, 0.0, 1.0],
//             },
//         };

//         let depth_clear_value = vk::ClearValue {
//             depth_stencil: vk::ClearDepthStencilValue {
//                 depth: 1.0,
//                 stencil: 0,
//             },
//         };

//         let clear_values = &[color_clear_value, depth_clear_value];
//         let info = vk::RenderPassBeginInfo::builder()
//             .render_pass(data.render_pass)
//             .framebuffer(data.framebuffers[i])
//             .render_area(render_area)
//             .clear_values(clear_values);

//         device.cmd_begin_render_pass(*command_buffer, &info, vk::SubpassContents::INLINE);
//         device.cmd_bind_pipeline(*command_buffer, vk::PipelineBindPoint::GRAPHICS, data.pipeline);
//         device.cmd_bind_vertex_buffers(*command_buffer, 0, &[data.vertex_buffer], &[0]);
//         device.cmd_bind_index_buffer(*command_buffer, data.index_buffer, 0, vk::IndexType::UINT32);
//         device.cmd_bind_descriptor_sets(
//             *command_buffer,
//             vk::PipelineBindPoint::GRAPHICS,
//             data.pipeline_layout,
//             0,
//             &[data.descriptor_sets[i]],
//             &[],
//         );
//         device.cmd_draw_indexed(*command_buffer, data.indices.len() as u32, 1, 0, 0, 0);
//         device.cmd_end_render_pass(*command_buffer);

//         device.end_command_buffer(*command_buffer)?;
//     }

//     Ok(())
// }

// //================================================
// // Sync Objects
// //================================================

// unsafe fn create_sync_objects(device: &Device, data: &mut AppData) -> Result<()> {
//     let semaphore_info = vk::SemaphoreCreateInfo::builder();
//     let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

//     for _ in 0..MAX_FRAMES_IN_FLIGHT {
//         data.image_available_semaphores
//             .push(device.create_semaphore(&semaphore_info, None)?);

//         data.in_flight_fences.push(device.create_fence(&fence_info, None)?);
//     }

//     for _ in 0..data.swapchain_images.len() {
//         data.render_finished_semaphores
//             .push(device.create_semaphore(&semaphore_info, None)?);
//     }

//     data.images_in_flight = data.swapchain_images.iter().map(|_| vk::Fence::null()).collect();

//     Ok(())
// }

// //================================================
// // Structs
// //================================================

// #[derive(Copy, Clone, Debug)]
// struct QueueFamilyIndices {
//     graphics: u32,
//     present: u32,
// }

// impl QueueFamilyIndices {
//     unsafe fn get(instance: &Instance, data: &AppData, physical_device: vk::PhysicalDevice) -> Result<Self> {
//         let properties = instance.get_physical_device_queue_family_properties(physical_device);

//         // Get graphics queue
//         let graphics = properties
//             .iter()
//             .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
//             .map(|i| i as u32);

//         // Get present queue
//         let mut present = None;
//         for (index, properties) in properties.iter().enumerate() {
//             if instance.get_physical_device_surface_support_khr(physical_device, index as u32, data.surface)? {
//                 present = Some(index as u32);
//                 break;
//             }
//         }

//         if let (Some(graphics), Some(present)) = (graphics, present) {
//             Ok(Self { graphics, present })
//         } else {
//             Err(anyhow!(SuitabilityError("Missing required queue families")))
//         }
//     }
// }

// #[derive(Clone, Debug)]
// struct SwapchainSupport {
//     capabilities: vk::SurfaceCapabilitiesKHR,
//     formats: Vec<vk::SurfaceFormatKHR>,
//     present_modes: Vec<vk::PresentModeKHR>,
// }

// impl SwapchainSupport {
//     unsafe fn get(instance: &Instance, data: &AppData, physical_device: vk::PhysicalDevice) -> Result<Self> {
//         Ok(Self {
//             capabilities: instance.get_physical_device_surface_capabilities_khr(physical_device, data.surface)?,
//             formats: instance.get_physical_device_surface_formats_khr(physical_device, data.surface)?,
//             present_modes: instance.get_physical_device_surface_present_modes_khr(physical_device, data.surface)?,
//         })
//     }
// }

// #[repr(C)]
// #[derive(Copy, Clone, Debug)]
// struct UniformBufferObject {
//     model: Mat4,
//     view: Mat4,
//     proj: Mat4,
// }

// #[repr(C)]
// #[derive(Copy, Clone, Debug, Default)]
// struct Vertex {
//     pos: Vec3,
//     color: Vec3,
//     normal: Vec3,
//     uv: Vec2,
// }

// impl Vertex {
//     const fn new(pos: Vec3, color: Vec3, normal: Vec3, uv: Vec2) -> Self {
//         Self { pos, color, normal, uv }
//     }

//     fn binding_description() -> vk::VertexInputBindingDescription {
//         vk::VertexInputBindingDescription::builder()
//             .binding(0)
//             .stride(size_of::<Vertex>() as u32)
//             .input_rate(vk::VertexInputRate::VERTEX)
//             .build()
//     }

//     fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
//         let pos = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(0)
//             .format(vk::Format::R32G32B32_SFLOAT)
//             .offset(0)
//             .build();

//         let color = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(1)
//             .format(vk::Format::R32G32B32_SFLOAT)
//             .offset(size_of::<Vec3>() as u32)
//             .build();

//         let normal = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(2)
//             .format(vk::Format::R32G32B32_SFLOAT)
//             .offset((size_of::<Vec3>() * 2) as u32)
//             .build();

//         let uv = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(3)
//             .format(vk::Format::R32G32_SFLOAT)
//             .offset((size_of::<Vec3>() * 3) as u32)
//             .build();

//         [pos, color, normal, uv]
//     }
// }

// #[repr(C)]
// #[derive(Copy, Clone, Debug, Default)]
// struct QuantizedVertex {
//     position: [u16; 3],
//     color: [u8; 3],
//     normal: [i8; 3],
//     uv: [u16; 2],
// }

// impl QuantizedVertex {

//     const fn from_slice(slice: &[u8; 16]) -> Self {
//         let position = [u16::from_le_bytes([slice[0], slice[1]]), u16::from_le_bytes([slice[2], slice[3]]), u16::from_le_bytes([slice[4], slice[5]])];
//         let color = [slice[6], slice[7], slice[8]];
//         let normal = [slice[9] as i8, slice[10] as i8, slice[11] as i8];
//         let uv = [u16::from_le_bytes([slice[12], slice[13]]), u16::from_le_bytes([slice[14], slice[15]])];

//         Self { position, color, normal, uv }
//     }

//     fn to_vertex(&self) -> Vertex {
//         let position: Vec3 = vec3(
//             meshopt::dequantize_half(self.position[0]),
//             meshopt::dequantize_half(self.position[1]),
//             meshopt::dequantize_half(self.position[2]),
//         );

//         let color: Vec3 = vec3(
//             self.color[0] as f32 / u8::MAX as f32,
//             self.color[1] as f32 / u8::MAX as f32,
//             self.color[2] as f32 / u8::MAX as f32,
//         );

//         let normal: Vec3 = vec3(
//             self.normal[0] as f32 / i8::MAX as f32,
//             self.normal[1] as f32 / i8::MAX as f32,
//             self.normal[2] as f32 / i8::MAX as f32,
//         );

//         let uv: Vec2 = vec2(
//             self.uv[0] as f32 / u16::MAX as f32,
//             self.uv[1] as f32 / u16::MAX as f32,
//         );

//         Vertex {
//             pos: position,
//             color: color,
//             normal: normal,
//             uv: uv,
//         }
//     }

//     fn binding_description() -> vk::VertexInputBindingDescription {
//         vk::VertexInputBindingDescription::builder()
//             .binding(0)
//             .stride(size_of::<QuantizedVertex>() as u32)
//             .input_rate(vk::VertexInputRate::VERTEX)
//             .build()
//     }

//     fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        
//         // Defaults to FORMAT_R16G16B16_SFLOAT but falls back to FORMAT_R16G16B16A16_SFLOAT
//         let pos = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(0)
//             .format(vk::Format::R16G16B16_SFLOAT)
//             .offset(0)
//             .build();

//         // Defaults to FORMAT_R8G8B8_UNORM but falls back to FORMAT_R8G8B8A8_UNORM
//         let color = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(1)
//             .format(vk::Format::R8G8B8_UNORM)
//             .offset(size_of::<[u16; 3]>() as u32)
//             .build();

//         // Defaults to FORMAT_R8G8B8_SNORM but falls back to FORMAT_R8G8B8A8_SNORM
//         let normal = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(2)
//             .format(vk::Format::R8G8B8_SNORM)
//             .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>()) as u32)
//             .build();
        
//         // Defaults to FORMAT_R16G16_UNORM but falls back to FORMAT_R16G16_SFLOAT
//         let uv = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(3)
//             .format(vk::Format::R16G16_UNORM)
//             .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>() + size_of::<[i8; 3]>()) as u32)
//             .build();

//         [pos, color, normal, uv]
//     }

//     fn attribute_descriptions_with_fallback(instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<[vk::VertexInputAttributeDescription; 4]> {
//         // Try preferred format, fall back if not supported
//         // Vertex attributes typically support FORMAT_VERTEX_BUFFER feature
//         let features = vk::FormatFeatureFlags::VERTEX_BUFFER;
        
//         // Position: Try R16G16B16_SFLOAT, fall back to R16G16B16A16_SFLOAT
//         let pos_format = get_supported_vertex_format(
//             instance,
//             physical_device,
//             &[vk::Format::R16G16B16_SFLOAT, vk::Format::R16G16B16A16_SFLOAT],
//             features,
//         )?;
//         let pos = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(0)
//             .format(pos_format)
//             .offset(0)
//             .build();

//         // Color: Try R8G8B8_UNORM, fall back to R8G8B8A8_UNORM
//         let color_format = get_supported_vertex_format(
//             instance,
//             physical_device,
//             &[vk::Format::R8G8B8_UNORM, vk::Format::R8G8B8A8_UNORM],
//             features,
//         )?;
//         let color = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(1)
//             .format(color_format)
//             .offset(size_of::<[u16; 3]>() as u32)
//             .build();

//         // Normal: Try R8G8B8_SNORM, fall back to R8G8B8A8_SNORM
//         let normal_format = get_supported_vertex_format(
//             instance,
//             physical_device,
//             &[vk::Format::R8G8B8_SNORM, vk::Format::R8G8B8A8_SNORM],
//             features,
//         )?;
//         let normal = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(2)
//             .format(normal_format)
//             .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>()) as u32)
//             .build();
        
//         // UV: Try R16G16_UNORM, fall back to R16G16_SFLOAT
//         let uv_format = get_supported_vertex_format(
//             instance,
//             physical_device,
//             &[vk::Format::R16G16_UNORM, vk::Format::R16G16_SFLOAT],
//             features,
//         )?;
//         let uv = vk::VertexInputAttributeDescription::builder()
//             .binding(0)
//             .location(3)
//             .format(uv_format)
//             .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>() + size_of::<[i8; 3]>()) as u32)
//             .build();

//         Ok([pos, color, normal, uv])
//     }
// }

// //================================================
// // Shared (Buffers)
// //================================================

// unsafe fn create_buffer(
//     instance: &Instance,
//     device: &Device,
//     data: &AppData,
//     size: vk::DeviceSize,
//     usage: vk::BufferUsageFlags,
//     properties: vk::MemoryPropertyFlags,
// ) -> Result<(vk::Buffer, vk::DeviceMemory)> {
//     // Buffer

//     let buffer_info = vk::BufferCreateInfo::builder()
//         .size(size)
//         .usage(usage)
//         .sharing_mode(vk::SharingMode::EXCLUSIVE);

//     let buffer = device.create_buffer(&buffer_info, None)?;

//     // Memory

//     let requirements = device.get_buffer_memory_requirements(buffer);

//     let memory_info = vk::MemoryAllocateInfo::builder()
//         .allocation_size(requirements.size)
//         .memory_type_index(get_memory_type_index(instance, data, properties, requirements)?);

//     let buffer_memory = device.allocate_memory(&memory_info, None)?;

//     device.bind_buffer_memory(buffer, buffer_memory, 0)?;

//     Ok((buffer, buffer_memory))
// }

// unsafe fn copy_buffer(
//     device: &Device,
//     data: &AppData,
//     source: vk::Buffer,
//     destination: vk::Buffer,
//     size: vk::DeviceSize,
// ) -> Result<()> {
//     let command_buffer = begin_single_time_commands(device, data)?;

//     let regions = vk::BufferCopy::builder().size(size);
//     device.cmd_copy_buffer(command_buffer, source, destination, &[regions]);

//     end_single_time_commands(device, data, command_buffer)?;

//     Ok(())
// }

// //================================================
// // Shared (Images)
// //================================================

// unsafe fn create_image(
//     instance: &Instance,
//     device: &Device,
//     data: &AppData,
//     width: u32,
//     height: u32,
//     mipmap_levels: u32,
//     samples: vk::SampleCountFlags,
//     format: vk::Format,
//     tiling: vk::ImageTiling,
//     usage: vk::ImageUsageFlags,
//     properties: vk::MemoryPropertyFlags,
// ) -> Result<(vk::Image, vk::DeviceMemory)> {
//     // Image

//     let info = vk::ImageCreateInfo::builder()
//         .image_type(vk::ImageType::_2D)
//         .extent(vk::Extent3D {
//             width,
//             height,
//             depth: 1
//         })
//         .mip_levels(mipmap_levels)
//         .array_layers(1)
//         .format(format)
//         .tiling(tiling)
//         .initial_layout(vk::ImageLayout::UNDEFINED)
//         .usage(usage)
//         .sharing_mode(vk::SharingMode::EXCLUSIVE)
//         .samples(samples);

//     let image = device.create_image(&info, None)?;

//     // Memory

//     let requirements = device.get_image_memory_requirements(image);

//     let info = vk::MemoryAllocateInfo::builder()
//         .allocation_size(requirements.size)
//         .memory_type_index(get_memory_type_index(instance, data, properties, requirements)?);

//     let image_memory = device.allocate_memory(&info, None)?;

//     device.bind_image_memory(image, image_memory, 0)?;

//     Ok((image, image_memory))
// }

// unsafe fn create_image_view(
//     device: &Device,
//     image: vk::Image,
//     format: vk::Format,
//     aspects: vk::ImageAspectFlags,
//     mipmap_levels: u32,
// ) -> Result<vk::ImageView> {
//     let subresource_range = vk::ImageSubresourceRange::builder()
//         .aspect_mask(aspects)
//         .base_mip_level(0)
//         .level_count(mipmap_levels)
//         .base_array_layer(0)
//         .layer_count(1);

//     let info = vk::ImageViewCreateInfo::builder()
//         .image(image)
//         .view_type(vk::ImageViewType::_2D)
//         .format(format)
//         .subresource_range(subresource_range);

//     Ok(device.create_image_view(&info, None)?)
// }

// unsafe fn transition_image_layout(
//     device: &Device,
//     data: &AppData,
//     image: vk::Image,
//     format: vk::Format,
//     old_layout: vk::ImageLayout,
//     new_layout: vk::ImageLayout,
//     mipmap_levels: u32,
// ) -> Result<()> {
//     let (src_access_mask, dst_access_mask, src_stage_mask, dst_stage_mask) = match (old_layout, new_layout) {
//         (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
//             vk::AccessFlags::empty(),
//             vk::AccessFlags::TRANSFER_WRITE,
//             vk::PipelineStageFlags::TOP_OF_PIPE,
//             vk::PipelineStageFlags::TRANSFER,
//         ),
//         (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
//             vk::AccessFlags::TRANSFER_WRITE,
//             vk::AccessFlags::SHADER_READ,
//             vk::PipelineStageFlags::TRANSFER,
//             vk::PipelineStageFlags::FRAGMENT_SHADER,
//         ),
//         _ => return Err(anyhow!("Unsupported image layout transition!")),
//     };

//     let command_buffer = begin_single_time_commands(device, data)?;

//     let subresource = vk::ImageSubresourceRange::builder()
//         .aspect_mask(vk::ImageAspectFlags::COLOR)
//         .base_mip_level(0)
//         .level_count(mipmap_levels)
//         .base_array_layer(0)
//         .layer_count(1);

//     let barrier = vk::ImageMemoryBarrier::builder()
//         .old_layout(old_layout)
//         .new_layout(new_layout)
//         .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
//         .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
//         .image(image)
//         .subresource_range(subresource)
//         .src_access_mask(src_access_mask)
//         .dst_access_mask(dst_access_mask);

//     device.cmd_pipeline_barrier(
//         command_buffer,
//         src_stage_mask,
//         dst_stage_mask,
//         vk::DependencyFlags::empty(),
//         &[] as &[vk::MemoryBarrier],
//         &[] as &[vk::BufferMemoryBarrier],
//         &[barrier],
//     );

//     end_single_time_commands(device, data, command_buffer)?;

//     Ok(())
// }

// unsafe fn copy_buffer_to_image(
//     device: &Device,
//     data: &AppData,
//     buffer: vk::Buffer,
//     image: vk::Image,
//     width: u32,
//     height: u32,
// ) -> Result<()> {
//     let command_buffer = begin_single_time_commands(device, data)?;

//     let subresource = vk::ImageSubresourceLayers::builder()
//     .aspect_mask(vk::ImageAspectFlags::COLOR)
//     .mip_level(0)
//     .base_array_layer(0)
//     .layer_count(1);

//     let region = vk::BufferImageCopy::builder()
//         .buffer_offset(0)
//         .buffer_row_length(0)
//         .buffer_image_height(0)
//         .image_subresource(subresource)
//         .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
//         .image_extent(vk::Extent3D {
//             width,
//             height,
//             depth: 1,
//         });

//     device.cmd_copy_buffer_to_image(
//         command_buffer,
//         buffer,
//         image,
//         vk::ImageLayout::TRANSFER_DST_OPTIMAL,
//         &[region],
//     );

//     end_single_time_commands(device, data, command_buffer)?;

//     Ok(())
// }

// //================================================
// // Shared (Other)
// //================================================

// unsafe fn get_memory_type_index(
//     instance: &Instance,
//     data: &AppData,
//     properties: vk::MemoryPropertyFlags,
//     requirements: vk::MemoryRequirements,
// ) -> Result<u32> {
//     let memory = instance.get_physical_device_memory_properties(data.physical_device);
//     (0..memory.memory_type_count)
//         .find(|i| {
//             let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
//             let memory_type = memory.memory_types[*i as usize];
//             suitable && memory_type.property_flags.contains(properties)
//         })
//         .ok_or_else(|| anyhow!("Failed to find suitable memory type"))
// }

// unsafe fn begin_single_time_commands(device: &Device, data: &AppData) -> Result<vk::CommandBuffer> {
//     // Allocate

//     let info = vk::CommandBufferAllocateInfo::builder()
//         .level(vk::CommandBufferLevel::PRIMARY)
//         .command_pool(data.command_pool)
//         .command_buffer_count(1);

//     let command_buffer = device.allocate_command_buffers(&info)?[0];

//     // Begin

//     let info = vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

//     device.begin_command_buffer(command_buffer, &info)?;

//     Ok(command_buffer)
// }

// unsafe fn end_single_time_commands(device: &Device, data: &AppData, command_buffer: vk::CommandBuffer) -> Result<()> {
//     // End

//     device.end_command_buffer(command_buffer)?;

//     // Submit

//     let command_buffers = &[command_buffer];
//     let info = vk::SubmitInfo::builder().command_buffers(command_buffers);

//     device.queue_submit(data.graphics_queue, &[info], vk::Fence::null())?;
//     device.queue_wait_idle(data.graphics_queue)?;

//     // Cleanup

//     device.free_command_buffers(data.command_pool, &[command_buffer]);

//     Ok(())
// }