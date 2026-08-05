use crate::engine::DeviceQueueHandle;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct DeviceContext {
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub physical_device: vk::PhysicalDevice,
    pub device_queue_handle: DeviceQueueHandle,
}

// #![allow(
//     dead_code,
//     unsafe_op_in_unsafe_fn,
//     unused_variables,
//     clippy::manual_slice_size_calculation,
//     clippy::too_many_arguments,
//     clippy::unnecessary_wraps
// )]

// use anyhow::{anyhow, Result};
// use log::*;
// use std::collections::HashSet;
// use std::ffi::CStr;
// use std::fmt::Debug;
// use std::os::raw::c_void;
// use thiserror::Error;
// use vulkanalia::loader::{LIBRARY, LibloadingLoader};
// use vulkanalia::prelude::v1_0::*;
// use vulkanalia::Version;
// use vulkanalia::vk::ExtDebugUtilsExtensionInstanceCommands;
// use vulkanalia::vk::KhrSurfaceExtensionInstanceCommands;
// use vulkanalia::window as vk_window;
// use winit::window::Window;

// use crate::engine::{PresentEngineBuilder};

// /// Whether the validation layers should be enabled (only enabled if debug assertions flag is active)
// const VALIDATION_ENABLED: bool = cfg!(debug_assertions);

// /// The name of the validation layers
// const VALIDATION_LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

// /// The required device extensions.
// const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];

// /// The Vulkan SDK version that started requiring the portability subset extension for macOS.
// const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);

// #[derive(Clone)]
// pub struct DeviceContext {
//     pub messenger: vk::DebugUtilsMessengerEXT,
//     pub entry: Entry,
//     pub instance: Instance,
//     pub device: Device,
//     pub physical_device: vk::PhysicalDevice,
//     pub graphics_queue: vk::Queue,
//     pub present_queue: vk::Queue,
//     pub graphics_queue_family_index: u32,
//     pub present_queue_family_index: u32,
// }

// impl DeviceContext {
//     pub fn new(present_engine_builder: &mut PresentEngineBuilder) -> Result<Self> {
//         unsafe {
//             // Dev-only logger with a sensible default; RUST_LOG still overrides this.
//             #[cfg(debug_assertions)]
//             {
//                 let mut logger = pretty_env_logger::formatted_builder();
//                 logger.parse_filters("info");
//                 logger.parse_default_env();
//                 logger.init();
//             }

//             let entry = DeviceContextBuilder::create_entry()?;
//             let (instance, messenger) = DeviceContextBuilder::create_instance(present_engine_builder.0.window.as_ref().unwrap(), &entry)?;

//             present_engine_builder.create_surface(instance.clone())?;

//             let physical_device = DeviceContextBuilder::pick_physical_device(instance.clone(), present_engine_builder.0.surface)?;
//             let device_context = DeviceContextBuilder::create_logical_device(messenger, &entry, &instance, physical_device, present_engine_builder.0.surface)?;
//             Ok(device_context)
//         }
//     }

//     pub fn get_memory_type_index(&self, properties: vk::MemoryPropertyFlags, requirements: vk::MemoryRequirements) -> Result<u32> {
//         unsafe {
//             let memory = self.instance.get_physical_device_memory_properties(self.physical_device);
//             (0..memory.memory_type_count)
//                 .find(|i| {
//                     let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
//                     let memory_type = memory.memory_types[*i as usize];
//                     suitable && memory_type.property_flags.contains(properties)
//                 })
//                 .ok_or_else(|| anyhow!("Failed to find suitable memory type"))
//         }
//     }

//     pub fn get_queue_family_indices(instance: &Instance, physical_device: vk::PhysicalDevice, surface: &vk::SurfaceKHR) -> Result<(u32, u32)> {
//         unsafe {
//             let properties = instance.get_physical_device_queue_family_properties(physical_device);

//             // Get graphics queue
//             let graphics = properties
//                 .iter()
//                 .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
//                 .map(|i| i as u32);

//             // Get present queue
//             let mut present = None;
//             for (index, properties) in properties.iter().enumerate() {
//                 if instance.get_physical_device_surface_support_khr(physical_device, index as u32, *surface)? {
//                     present = Some(index as u32);
//                     break;
//                 }
//             }

//             if let (Some(graphics), Some(present)) = (graphics, present) {
//                 Ok((graphics, present))
//             } else {
//                 Err(anyhow!(SuitabilityError("Missing required queue families")))
//             }
//         }
//     }

//     pub fn check_physical_device(instance: &Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> Result<()> {
//         DeviceContext::get_queue_family_indices(instance, physical_device, &surface)?;
//         DeviceContext::check_physical_device_extensions(instance, physical_device)?;

//         let available_formats = unsafe { instance.get_physical_device_surface_formats_khr(physical_device, surface).unwrap() };
//         let available_present_modes = unsafe { instance.get_physical_device_surface_present_modes_khr(physical_device, surface).unwrap() };

//         if available_formats.is_empty() || available_present_modes.is_empty() {
//             return Err(anyhow!(SuitabilityError("Insufficient swapchain support")))
//         }

//         let features = unsafe { instance.get_physical_device_features(physical_device) };
//         if features.sampler_anisotropy != vk::TRUE {
//             return Err(anyhow!(SuitabilityError("No sampler anisotropy")));
//         }

//         Ok(())
//     }

//     fn check_physical_device_extensions(instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<()> {
//         unsafe {
//             let extensions = instance
//                 .enumerate_device_extension_properties(physical_device, None)?
//                 .iter()
//                 .map(|e| e.extension_name)
//                 .collect::<HashSet<_>>();

//             if DEVICE_EXTENSIONS.iter().all(|e| extensions.contains(e)) {
//                 Ok(())
//             } else {
//                 Err(anyhow!(SuitabilityError("Missing required device extensions")))
//             }
//         }
//     }
// }

// struct DeviceContextBuilder;

// impl DeviceContextBuilder {
//     pub unsafe fn create_entry() -> Result<Entry> {
//         let loader = LibloadingLoader::new(LIBRARY)?;
//         Entry::new(loader).map_err(|b| anyhow!("{}", b))
//     }

//     pub unsafe fn create_instance(window: &Window, entry: &Entry) -> Result<(Instance, vk::DebugUtilsMessengerEXT)> {
//         // Application Info

//         let application_info = vk::ApplicationInfo::builder()
//             .application_name(b"Vulkan Tutorial\0")
//             .application_version(vk::make_version(1, 0, 0))
//             .engine_name(b"No Engine\0")
//             .engine_version(vk::make_version(1, 0, 0))
//             .api_version(vk::make_version(1, 1, 0));

//         // Layers

//         let available_layers = entry
//             .enumerate_instance_layer_properties()?
//             .iter()
//             .map(|l| l.layer_name)
//             .collect::<HashSet<_>>();

//         if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
//             return Err(anyhow!("Validation layer requested but not supported"));
//         }

//         let layers = if VALIDATION_ENABLED {
//             vec![VALIDATION_LAYER.as_ptr()]
//         } else {
//             Vec::new()
//         };

//         // Extensions

//         // Get global required extensions for Vulkan to run
//         let mut extensions = vk_window::get_required_instance_extensions(window)
//             .iter()
//             .map(|e| e.as_ptr())
//             .collect::<Vec<_>>();

//         // Add macOS required extensions if user is on macOS
//         let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
//             info!("Enabling extensions for macOS portability");
//             extensions.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION.name.as_ptr());
//             extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
//             vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
//         } else {
//             vk::InstanceCreateFlags::empty()
//         };

//         if VALIDATION_ENABLED {
//             extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
//         }

//         // Create

//         let mut info = vk::InstanceCreateInfo::builder()
//             .application_info(&application_info)
//             .enabled_layer_names(&layers)
//             .enabled_extension_names(&extensions)
//             .flags(flags);

//         let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
//             .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
//             .message_type(
//                 vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
//                 | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
//                 | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
//             )
//             .user_callback(Some(DeviceContextBuilder::debug_callback));

//         if VALIDATION_ENABLED {
//             info = info.push_next(&mut debug_info);
//         }

//         let instance = entry.create_instance(&info, None)?;

//         // Messenger
//         let mut messenger = vk::DebugUtilsMessengerEXT::null();

//         if VALIDATION_ENABLED {
//             messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
//         }

//         Ok((instance, messenger))
//     }

//     extern "system" fn debug_callback(
//         severity: vk::DebugUtilsMessageSeverityFlagsEXT,
//         type_: vk::DebugUtilsMessageTypeFlagsEXT,
//         data: *const vk::DebugUtilsMessengerCallbackDataEXT,
//         _: *mut c_void,
//     ) -> vk::Bool32 {
//         let data = unsafe { *data };
//         let message = unsafe { CStr::from_ptr(data.message) }.to_string_lossy();

//         if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
//             error!("({:?}) {}", type_, message);
//         } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
//             warn!("({:?}) {}", type_, message);
//         } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
//             debug!("({:?}) {}", type_, message);
//         } else {
//             trace!("({:?}) {}", type_, message);
//         }

//         vk::FALSE
//     }

//     pub unsafe fn pick_physical_device(instance: Instance, surface: vk::SurfaceKHR) -> Result<vk::PhysicalDevice> {
//         let chosen_physical_device = Some(*instance.enumerate_physical_devices()?
//             .iter()
//             .filter_map(|p| {
//                 let properties = instance.get_physical_device_properties(*p);
//                 if let Err(error) = DeviceContext::check_physical_device(&instance, *p, surface) {
//                     warn!("Skipping physical device ('{}'): {}", properties.device_name, error);
//                     None
//                 } else {
//                     info!("Found available physical device ('{}')\n\tDevice type ('{:?}')\n\tPush constant size ({})\n\tMax image dimension 2d ({})",
//                     properties.device_name,
//                     properties.device_type,
//                     properties.limits.max_push_constants_size,
//                     properties.limits.max_image_dimension_2d,
//                 );
//                     Some(p)
//                 }
//             })
//             .max_by_key(|p| {
//                 // lower score for preferred device types
//                 let properties = instance.get_physical_device_properties(**p);

//                 match properties.device_type {
//                     vk::PhysicalDeviceType::DISCRETE_GPU => 10000 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                     vk::PhysicalDeviceType::INTEGRATED_GPU => 1000 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                     vk::PhysicalDeviceType::VIRTUAL_GPU => 100 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                     vk::PhysicalDeviceType::CPU => 10 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                     vk::PhysicalDeviceType::OTHER => 1 + (properties.limits.max_push_constants_size / 32) + (properties.limits.max_image_dimension_2d / 8192),
//                     _ => 0
//                 }

//             })
//             .unwrap());

//         if chosen_physical_device != None {
//             info!("Chose physical device ('{}')", instance.get_physical_device_properties(chosen_physical_device.unwrap()).device_name);
//             return Ok(chosen_physical_device.unwrap());
//         }

//         Err(anyhow!("Failed to find suitable physical device"))
//     }

//     pub unsafe fn create_logical_device(messenger: vk::DebugUtilsMessengerEXT, entry: &Entry, instance: &Instance, physical_device: vk::PhysicalDevice, surface: vk::SurfaceKHR) -> Result<DeviceContext> {
//         // Queue Create Infos

//         let (graphics_index, present_index) = DeviceContext::get_queue_family_indices(instance, physical_device, &surface)?;

//         let mut unique_indices = HashSet::new();
//         unique_indices.insert(graphics_index);
//         unique_indices.insert(present_index);

//         let queue_priorities = &[1.0];
//         let queue_infos = unique_indices
//             .iter()
//             .map(|i| {
//                 vk::DeviceQueueCreateInfo::builder()
//                 .queue_family_index(*i)
//                 .queue_priorities(queue_priorities)
//             })
//             .collect::<Vec<_>>();

//         // Layers

//         let layers = if VALIDATION_ENABLED {
//             vec![VALIDATION_LAYER.as_ptr()]
//         } else {
//             vec![]
//         };

//         // Extensions

//         let mut extensions = DEVICE_EXTENSIONS
//             .iter()
//             .map(|n| n.as_ptr())
//             .collect::<Vec<_>>();

//         // Required by Vulkan SDK on macOS
//         if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
//             extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
//         }

//         // Enforce shader draw parameters for slang shaders
//         extensions.push(vk::KHR_SHADER_DRAW_PARAMETERS_EXTENSION.name.as_ptr());

//         // Enable descriptor indexing for bindless rendering
//         extensions.push(vk::EXT_DESCRIPTOR_INDEXING_EXTENSION.name.as_ptr());

//         // Features

//         let features = vk::PhysicalDeviceFeatures::builder()
//             .sampler_anisotropy(true)
//             .sample_rate_shading(true)
//             .shader_int16(true)
//             .multi_draw_indirect(true);

//         let mut descriptor_indexing_features = vk::PhysicalDeviceVulkan12Features::builder()
//             .descriptor_indexing(true)
//             .descriptor_binding_sampled_image_update_after_bind(true)
//             .descriptor_binding_partially_bound(true)
//             .runtime_descriptor_array(true);

//         let mut storage_16bit_features = vk::PhysicalDevice16BitStorageFeatures::builder()
//             .storage_buffer_16bit_access(true)
//             .uniform_and_storage_buffer_16bit_access(true);

//         // Create

//         let info = vk::DeviceCreateInfo::builder()
//             .queue_create_infos(&queue_infos)
//             .enabled_layer_names(&layers)
//             .enabled_extension_names(&extensions)
//             .enabled_features(&features)
//             .push_next(&mut storage_16bit_features)
//             .push_next(&mut descriptor_indexing_features);

//         let device = instance.create_device(physical_device, &info, None)?;

//         // Queues

//         let graphics_queue = device.get_device_queue(graphics_index, 0);
//         let present_queue = device.get_device_queue(present_index, 0);

//         Ok(DeviceContext {
//             messenger,
//             entry: entry.clone(),
//             instance: instance.clone(),
//             device,
//             physical_device,
//             graphics_queue,
//             present_queue,
//             graphics_queue_family_index: graphics_index,
//             present_queue_family_index: present_index,
//         })
//     }
// }

// #[derive(Debug, Error)]
// #[error("{0}")]
// pub struct SuitabilityError(pub &'static str);
