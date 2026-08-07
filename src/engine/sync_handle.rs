use vulkanalia::prelude::v1_0::*;

#[derive(Clone)]
pub struct SyncHandle {
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub max_frames_in_flight: usize,
    pub current_frame: usize,
}
