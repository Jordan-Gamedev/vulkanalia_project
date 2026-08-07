use crate::engine::SwapchainHandle;
use crate::engine::Texture;
use crate::engine::WindowHandle;
use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct PresentHandle {
    pub window_handle: WindowHandle,
    pub swapchain_handle: SwapchainHandle,
    pub color_texture: Texture,
    pub depth_texture: Texture,
    pub msaa_samples: vk::SampleCountFlags,
}
