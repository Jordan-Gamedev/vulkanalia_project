#![allow(
    dead_code,
    unsafe_op_in_unsafe_fn,
    unused_variables,
    clippy::manual_slice_size_calculation,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use anyhow::{anyhow, Result};
use crate::engine::DescriptorHandle;
use crate::engine::DeviceQueueHandle;
use crate::engine::QuantizedVertex;
use crate::engine::SwapchainHandle;
use crate::engine::Texture;
use crate::engine::WindowHandle;
use crate::engine::buffers::Buffer;
use crate::resources::AssetId;
use log::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::ffi::CStr;
use std::fmt::Debug;
use std::os::raw::c_void;
use std::sync::Arc;
use thiserror::Error;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LIBRARY, LibloadingLoader};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::Version;
use vulkanalia::vk::ExtDebugUtilsExtensionInstanceCommands;
use vulkanalia::vk::{KhrSurfaceExtensionInstanceCommands, KhrSwapchainExtensionDeviceCommands};
use vulkanalia::window as vk_window;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Fullscreen, Window, WindowBuilder};

pub struct VulkanRenderer {
    // Device Context
    
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub physical_device: vk::PhysicalDevice,
    pub device_queue_handle: DeviceQueueHandle,

    // Present Engine
    pub window_handle: WindowHandle,
    pub swapchain_handle: SwapchainHandle,
    pub color_texture: Texture,
    pub depth_texture: Texture,
    pub msaa_samples: vk::SampleCountFlags,

    // Render Pipeline Engine
    
    pub base_render_pass: vk::RenderPass,
    pub descriptor_handle: DescriptorHandle,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub framebuffers: Vec<vk::Framebuffer>,

    // Command Engine

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
    pub indirect_draw_buffer_mapped: *mut IndirectDrawData,
    pub indirect_draw_capacity: usize,

    pub instance_buffer: vk::Buffer,
    pub instance_buffer_memory: vk::DeviceMemory,
    pub instance_buffer_mapped: *mut PerInstanceData,
    pub instance_capacity: usize,

    // Model Engine

    pub vertex_buffer: Buffer<QuantizedVertex>,
    pub index_buffer: Buffer<u32>,    
    pub uniform_buffers: Vec<Buffer<UniformBufferObject>>,
    pub dyn_model_matrix_buffer: Buffer<QuantizedModelMatrix>,
    pub static_model_matrix_buffer: Buffer<QuantizedModelMatrix>,
    pub loaded_models: HashMap<(AssetId, AssetId), Model>,

    // Texture Engine

    loaded_textures: HashMap<AssetId, Texture>,
    available_texture_slots: Vec<u32>,
    samplers: HashMap<SamplerContents, SamplerUsage>,
    available_sampler_slots: Vec<u32>,
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
            let surface = create_surface(instance, &window)?;

            // Get physical Device
            let physical_device = pick_physical_device(instance, surface)?;

            // Create logical device
            let device = create_logical_device(messenger, &entry, &instance, physical_device, surface)?;

            // Get device queues
            let device_queue_handle = get_device_graphics_present_queues(device, &instance, physical_device, &surface)?;

            // Set a starting value for multisample antialiasing
            let msaa_samples = set_default_msaa(instance, physical_device);

            // Create the window's swapchain
            let swapchain_handle = create_swapchain(instance, &window, physical_device, device, surface, device_queue_handle)?;
        
            // Create screen color texture
            let color_texture = create_color_texture(&swapchain_handle, msaa_samples, device)?;
            
            // Create screen depth texture
            let depth_texture = create_depth_texture(instance, &swapchain_handle, msaa_samples, device, physical_device)?;

            // Create base render pass
            let base_render_pass = create_base_render_pass(instance, device, physical_device, &swapchain_handle, msaa_samples)?;
        
            // Create a descriptor set layout for gpu objects
            let descriptor_set_layout = create_descriptor_set_layout(device)?;

            // Create a descriptor pool
            let descriptor_pool = create_descriptor_pool(device)?;
        
            // Create the render pipeline
            let (pipeline, pipeline_layout) = create_pipeline(&swapchain_handle, msaa_samples, descriptor_set_layout, base_render_pass, &instance, &device)?;
        
            // Create framebuffers
            let framebuffers = create_framebuffers(&swapchain_handle, color_texture, depth_texture, base_render_pass, device)?;
        }

        unsafe {
            // Create command pool
            command_engine_builder.create_command_pool(app.device_context.as_ref().clone().unwrap())?;

            // Create indirect draw buffer
            command_engine_builder.create_indirect_draw_buffer(app.device_context.as_ref().clone().unwrap())?;

            // Create instance data buffer
            command_engine_builder.create_instance_buffer(app.device_context.as_ref().clone().unwrap())?;

            // Create vertex and index buffers
            model_engine_builder.create_vertex_index_buffers(app.device_context.as_ref().clone().unwrap(), &command_engine_builder.0);

            // Create uniform buffer objects
            model_engine_builder.create_uniform_buffers(app.device_context.as_ref().clone().unwrap(), command_engine_builder.0.clone())?;

            // Create model matrix storage buffer
            model_engine_builder.create_model_matrix_buffers(app.device_context.as_ref().clone().unwrap(), &app.command_engine)?;

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
        }

        Ok(app)
    }

    pub fn run(&mut self, bevy_app: &mut bevy_app::App) {
        PresentEngine::update_window(self, bevy_app).unwrap();
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

// ______________________________________________________________________________________________________________________________________________________
// Device Context Setup
// ______________________________________________________________________________________________________________________________________________________

/// Whether the validation layers should be enabled (only enabled if debug assertions flag is active)
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

/// The name of the validation layers
const VALIDATION_LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

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

unsafe fn create_instance(window: &Window, entry: &Entry) -> Result<(Instance, vk::DebugUtilsMessengerEXT)> {
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
        extensions.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION.name.as_ptr());
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
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
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

unsafe fn pick_physical_device(instance: Instance, surface: vk::SurfaceKHR) -> Result<vk::PhysicalDevice> {
    let chosen_physical_device = Some(*instance.enumerate_physical_devices()?
        .iter()
        .filter_map(|p| {
            let properties = instance.get_physical_device_properties(*p);
            if let Err(error) = DeviceContext::check_physical_device(&instance, *p, surface) {
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
        info!("Chose physical device ('{}')", instance.get_physical_device_properties(chosen_physical_device.unwrap()).device_name);
        return Ok(chosen_physical_device.unwrap());
    }

    Err(anyhow!("Failed to find suitable physical device"))
}

unsafe fn create_logical_device(messenger: vk::DebugUtilsMessengerEXT, entry: &Entry, instance: &Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> Result<Device> {
    // Queue Create Infos

    let (graphics_index, present_index) = get_queue_family_indices(instance, physical_device, &surface)?;
    
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
    
    // Layers

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

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
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features)
        .push_next(&mut storage_16bit_features)
        .push_next(&mut descriptor_indexing_features);

    let device = instance.create_device(physical_device, &info, None)?;

    Ok(device)
}

unsafe fn get_device_graphics_present_queues(device: Device, instance: &Instance, physical_device: vk::PhysicalDevice, surface: &vk::SurfaceKHR) -> Result<DeviceQueueHandle> {
    let (graphics_index, present_index) = get_queue_family_indices(instance, physical_device, surface)?;
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

fn check_physical_device(instance: &Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> Result<()> {
    get_queue_family_indices(instance, physical_device, &surface)?;
    check_physical_device_extensions(instance, physical_device)?;

    let available_formats = unsafe { instance.get_physical_device_surface_formats_khr(physical_device, surface).unwrap() };
    let available_present_modes = unsafe { instance.get_physical_device_surface_present_modes_khr(physical_device, surface).unwrap() };

    if available_formats.is_empty() || available_present_modes.is_empty() {
        return Err(anyhow!(SuitabilityError("Insufficient swapchain support")))
    }

    let features = unsafe { instance.get_physical_device_features(physical_device) };
    if features.sampler_anisotropy != vk::TRUE {
        return Err(anyhow!(SuitabilityError("No sampler anisotropy")));
    }

    Ok(())
}

fn check_physical_device_extensions(instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<()> {
    unsafe {
        let extensions = instance
            .enumerate_device_extension_properties(physical_device, None)?
            .iter()
            .map(|e| e.extension_name)
            .collect::<HashSet<_>>();
    
        if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
            Ok(())
        } else {
            Err(anyhow!(SuitabilityError("Missing required device extensions")))
        }
    }
}

fn get_queue_family_indices(instance: &Instance, physical_device: vk::PhysicalDevice, surface: &vk::SurfaceKHR) -> Result<(u32, u32)> {
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
            if instance.get_physical_device_surface_support_khr(physical_device, index as u32, *surface)? {
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

fn get_memory_type_index(instance: &Instance, physical_device: vk::PhysicalDevice, properties: vk::MemoryPropertyFlags, requirements: vk::MemoryRequirements) -> Result<u32> {
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
        .with_inner_size(LogicalSize::new(2560, 1600))
        .build(&event_loop)
        .unwrap();

    // Set fullscreen if enabled
    if with_fullscreen && let Some(monitor) = window.current_monitor().or_else(|| window.primary_monitor()) {
        if let Some(video_mode) = monitor
            .video_modes()
            //.max_by_key(|mode| mode.refresh_rate_millihertz() + mode.size().width * mode.size().height)
            .find(|mode| {
                mode.refresh_rate_millihertz() / 1000 == 240 &&
                mode.size().width == 1920 &&
                mode.size().height == 1200
            })
        {
            window.set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())));

            println!("\nDisplay: {}x{}@{}Hz\n", video_mode.size().width, video_mode.size().height, video_mode.refresh_rate_millihertz() / 1000);
        }
    }

    Ok((event_loop, window))
}

unsafe fn create_surface(instance: Instance, window: &Window) -> Result<vk::SurfaceKHR> {
    // Surface
    Ok(vulkanalia::window::create_surface(&instance, window, window)?)
}

fn set_default_msaa(instance: Instance, physical_device: vk::PhysicalDevice) -> vulkanalia::vk::SampleCountFlags {
    let max_msaa = get_max_msaa_samples(instance, physical_device);
    let chosen_msaa = if max_msaa < vk::SampleCountFlags::_4 { max_msaa } else { vk::SampleCountFlags::_4 }; 
    info!("Max msaa detected: {:?}", max_msaa);
    info!("Chosen msaa: {:?}", chosen_msaa);
    chosen_msaa
}

unsafe fn create_swapchain(
    instance: Instance,
    window: &Window,
    physical_device: vk::PhysicalDevice,
    device: Device,
    surface: vk::SurfaceKHR,
    device_queue_handle: DeviceQueueHandle) -> Result<SwapchainHandle> {
    // Image

    let swapchain_capabilities = instance.get_physical_device_surface_capabilities_khr(physical_device, surface)?;
    
    let surface_format = get_swapchain_surface_format(instance, physical_device, surface);
    let present_mode = get_swapchain_present_mode(instance, physical_device, surface);
    let swapchain_extent = get_swapchain_extent(window, swapchain_capabilities);
    let swapchain_format = surface_format.format;

    let mut image_count = swapchain_capabilities.min_image_count + 1;
    if swapchain_capabilities.max_image_count != 0 && image_count > swapchain_capabilities.max_image_count {
        image_count = swapchain_capabilities.max_image_count
    }

    let mut queue_family_indices = vec![];
    let image_sharing_mode = if device_queue_handle.graphics_queue_family_index != device_queue_handle.present_queue_family_index {
        queue_family_indices.push(device_queue_handle.graphics_queue_family_index);
        queue_family_indices.push(device_queue_handle.present_queue_family_index);
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

    let swapchain = device.create_swapchain_khr(&info, None)?;

    // Images

    let swapchain_images = device.get_swapchain_images_khr(swapchain)?;

    // Image Views

    let swapchain_image_views = swapchain_images
        .iter()
        .map(|i| TextureEngine::create_image_view(device, *i, swapchain_format, vk::ImageAspectFlags::COLOR, 1))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SwapchainHandle {
        swapchain,
        images: swapchain_images,
        image_views: swapchain_image_views,
        format: swapchain_format,
        extent: swapchain_extent,
    })
}

unsafe fn create_color_texture(swapchain_handle: &SwapchainHandle, msaa_samples: vk::SampleCountFlags, device: Device) -> Result<Texture> {
    // Image + Image Memory

    let (color_image, color_image_memory) = TextureEngine::create_image(
        context.clone(),
        swapchain_handle.extent.width,
        swapchain_handle.extent.height,
        1,
        msaa_samples,
        swapchain_handle.format,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
        vk::MemoryPropertyFlags::DEVICE_LOCAL
    )?;

    let color_image = color_image;
    let color_image_memory = color_image_memory;

    // Image View

    let color_image_view = TextureEngine::create_image_view(
        device,
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

unsafe fn create_depth_texture(instance: Instance, swapchain_handle: &SwapchainHandle, msaa_samples: vk::SampleCountFlags, device: Device, physical_device: vk::PhysicalDevice) -> Result<Texture> {
    // Image + Image Memory

    let format = get_depth_format(instance, physical_device)?;

    let (depth_image, depth_image_memory) = TextureEngine::create_image(
        context.clone(),
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

    let depth_image_view = TextureEngine::create_image_view(device, depth_image, format, vk::ImageAspectFlags::DEPTH, 1)?;

    Ok(Texture {
        image: depth_image,
        image_memory: depth_image_memory,
        image_view: depth_image_view,
    })
}

// Helper Functions

fn get_max_msaa_samples(instance: Instance, physical_device: vk::PhysicalDevice) -> vk::SampleCountFlags {
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

unsafe fn get_swapchain_surface_format(instance: Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> vk::SurfaceFormatKHR {      
    let formats = instance.get_physical_device_surface_formats_khr(physical_device, surface).unwrap();
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

unsafe fn get_swapchain_present_mode(instance: Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> vk::PresentModeKHR {
    let present_modes = instance.get_physical_device_surface_present_modes_khr(physical_device, surface).unwrap();
    
    present_modes
        .iter()
        .cloned()
    .find(|m| *m == vk::PresentModeKHR::IMMEDIATE)
    .or_else(|| present_modes.iter().cloned().find(|m| *m == vk::PresentModeKHR::MAILBOX))
    .unwrap_or(vk::PresentModeKHR::FIFO)
}

fn get_depth_format(instance: Instance, physical_device: vk::PhysicalDevice) -> Result<vk::Format> {
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

/// The maximum number of frames that can be processed concurrently
const MAX_FRAMES_IN_FLIGHT: u32 = 4;

// Build Functions

unsafe fn create_base_render_pass(instance: Instance, device: Device, physical_device: vk::PhysicalDevice, swapchain_handle: &SwapchainHandle, msaa_samples: vk::SampleCountFlags) -> Result<vk::RenderPass> {
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
        .format(get_depth_format(instance, physical_device)?)
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

    let render_pass = device.create_render_pass(&info, None)?;

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
    let mut layout_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
        .binding_flags(binding_flags);

    let bindings = &[texture_binding, sampler_binding, ubo_binding, static_model_matrix_binding, dyn_model_matrix_binding, indirect_draw_binding, instance_data_binding];
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

    let pool_sizes = &[texture_size, sampler_size, ubo_size, static_model_matrix_size, dyn_model_matrix_size, indirect_draw_size, instance_data_size];
    let info = vk::DescriptorPoolCreateInfo::builder()
        .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
        .pool_sizes(pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT);

    let descriptor_pool = device.create_descriptor_pool(&info, None)?;

    Ok(descriptor_pool)
}

unsafe fn create_descriptor_sets(device: Device, descriptor_set_layout: vk::DescriptorSetLayout, descriptor_pool: vk::DescriptorPool) -> Result<()> {
    // Allocate

    let layouts = vec![descriptor_set_layout; MAX_FRAMES_IN_FLIGHT as usize];
    let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&layouts);

    let descriptor_sets = device.allocate_descriptor_sets(&info)?;

    // Update

    for i in 0..MAX_FRAMES_IN_FLIGHT {
        let info = vk::DescriptorBufferInfo::builder()
            .buffer(model_engine.uniform_buffers[i].buffer)
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
            .buffer(model_engine.static_model_matrix_buffer.buffer)
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
            .buffer(model_engine.dyn_model_matrix_buffer.buffer)
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

unsafe fn create_pipeline(swapchain_handle: &SwapchainHandle, msaa_samples: vk::SampleCountFlags, descriptor_set_layout: vk::DescriptorSetLayout, render_pass: vk::RenderPass, instance: &Instance, device: &Device, physical_device: vk::PhysicalDevice) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    // Stages

    let shader = include_bytes!("../../assets/shaders/shader.spv");
    
    let shader_module = create_shader_module(&device, &shader[..])?;

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
    let attribute_descriptions = QuantizedVertex::attribute_descriptions(&instance, &physical_device)?;
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
    let pipeline_layout = device.create_pipeline_layout(&layout_info, None)?;

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

    let pipeline = device
        .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)?
        .0[0];

    // Cleanup

    device.destroy_shader_module(shader_module, None);

    Ok((pipeline, pipeline_layout))
}

unsafe fn create_framebuffers(swapchain_handle: &SwapchainHandle, color_texture: Texture, depth_texture: Texture, render_pass: vk::RenderPass, device: Device) -> Result<Vec<vk::Framebuffer>> {
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

fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule> {
    unsafe {
        let bytecode = Bytecode::new(bytecode).unwrap();
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(bytecode.code())
            .code_size(bytecode.code_size());
        Ok(device.create_shader_module(&info, None)?)
    }
}