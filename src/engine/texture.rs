use vulkanalia::prelude::v1_0::*;

pub struct Texture {
    pub image: vk::Image,
    pub image_memory: vk::DeviceMemory,
    pub image_view: vk::ImageView,
}