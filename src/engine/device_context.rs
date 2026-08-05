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
