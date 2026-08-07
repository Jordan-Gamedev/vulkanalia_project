use std::sync::Arc;
use vulkanalia::prelude::v1_0::*;
use winit::event_loop::EventLoop;
use winit::window::Window;

#[derive(Clone)]
pub struct WindowHandle {
    pub window: Arc<Window>,
    pub event_loop: Option<Arc<EventLoop<()>>>,
    pub surface: vk::SurfaceKHR,
    pub is_resized: bool,
}

unsafe impl Sync for WindowHandle {}
unsafe impl Send for WindowHandle {}
