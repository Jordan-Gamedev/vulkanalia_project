use vulkanalia::prelude::v1_0::*;

pub struct DeviceQueueHandle {
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub graphics_queue_family_index: u32,
    pub present_queue_family_index: u32,
}