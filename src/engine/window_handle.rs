use std::sync::Arc;
use vulkanalia::prelude::v1_0::*;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct WindowHandle {
    pub window: Window,
    pub event_loop: Option<Arc<EventLoop<()>>>,
    pub surface: vk::SurfaceKHR,
    pub is_resized: bool,
}
