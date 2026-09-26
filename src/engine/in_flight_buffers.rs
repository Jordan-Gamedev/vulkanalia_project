// TODO: Implement delay for cases where you do not want duplicate data and can handle the changed being delayed by frames = max frames in flight

use crate::engine::Buffer;
use vulkanalia::prelude::v1_0::*;

use super::device_context::DeviceContext;

#[derive(Default)]
pub struct InFlightBuffers<T: Clone + std::fmt::Debug + Default> {
    pub buffers: Vec<Buffer<T>>,
    updated_buffer_frame_index: usize,
    num_frames_needing_update: usize,
    max_frames_in_flight: usize,
    current_frame_index: usize,
}

impl<T: Clone + std::fmt::Debug + Default> InFlightBuffers<T> {
    pub fn new(
        max_frames_in_flight: usize,
        device_context: &DeviceContext,
        command_pool: vk::CommandPool,
        initial_capacity: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        alloc_dealloc_threshold: u32,
        initial_contents: Vec<T>,
        should_preserve_indices: bool,
    ) -> Self {
        Self {
            buffers: vec![
                Buffer::new(
                    device_context,
                    command_pool,
                    initial_capacity,
                    usage,
                    properties,
                    alloc_dealloc_threshold,
                    initial_contents,
                    should_preserve_indices
                );
                max_frames_in_flight
            ],
            updated_buffer_frame_index: 0,
            num_frames_needing_update: 0,
            max_frames_in_flight,
            current_frame_index: 0,
        }
    }

    pub fn destroy(&mut self, device: &Device) {
        self.buffers.iter_mut().for_each(|b| b.destroy(device));
    }

    pub fn get_current(&self) -> &Buffer<T> {
        &self.buffers[self.current_frame_index]
    }

    pub fn get_current_mut(&mut self) -> &mut Buffer<T> {
        &mut self.buffers[self.current_frame_index]
    }

    // Call if you want the other frame's versions of the buffer to be overwritten by data from this buffer
    pub fn signal_buffer_change_propagation(&mut self) {
        self.num_frames_needing_update = self.buffers.len() - 1;
    }

    // If changed, at the beginning of the frame replace this buffer with the previous frame's buffer, which is the latest version of the buffer
    pub fn update(&mut self, device_context: &DeviceContext, command_pool: vk::CommandPool) {
        self.current_frame_index = (self.current_frame_index + 1) % self.max_frames_in_flight;
        if self.num_frames_needing_update > 0 {
            let contents = self.buffers[self.updated_buffer_frame_index]
                .get_buffer_items(device_context, command_pool, true)
                .unwrap();
            self.buffers[self.current_frame_index].recreate(device_context, command_pool, contents);
            self.num_frames_needing_update -= 1;
        }
        self.updated_buffer_frame_index = self.current_frame_index;
    }
}

// use crate::engine::Buffer;
// use vulkanalia::prelude::v1_0::*;

// use super::device_context::DeviceContext;

// type BufferCommand<T> = Box<dyn FnOnce(&mut Buffer<T>) + Send + Sync>;

// #[derive(Default)]
// pub struct InFlightBuffers<T: Clone + std::fmt::Debug + Default> {
//     pub buffers: Vec<Buffer<T>>,
//     command_queue: Vec<Vec<BufferCommand<T>>>,
//     current_frame_index: usize,
// }

// unsafe impl<T: Clone + std::fmt::Debug + Default> Sync for InFlightBuffers<T> {}
// unsafe impl<T: Clone + std::fmt::Debug + Default> Send for InFlightBuffers<T> {}

// impl<T: Clone + std::fmt::Debug + Default> InFlightBuffers<T> {
//     /// buffer_count = MAX_FRAMES_IN_FLIGHT if you want duplicated data per frame whose changes are reflected instantly
//     ///
//     /// buffer_count = 1 if you plan on using command delaying in the case you prefer delayed changes over duplicated data
//     pub fn new(
//         max_frames_in_flight: usize,
//         buffer_count: usize,
//         device_context: &DeviceContext,
//         command_pool: vk::CommandPool,
//         initial_capacity: vk::DeviceSize,
//         usage: vk::BufferUsageFlags,
//         properties: vk::MemoryPropertyFlags,
//         alloc_dealloc_threshold: u32,
//         initial_contents: Vec<T>,
//         should_preserve_indices: bool,
//     ) -> Self {
//         let mut command_queue: Vec<Vec<BufferCommand<T>>> = Vec::new();
//         for _ in 0..max_frames_in_flight {
//             command_queue.push(Vec::new());
//         }

//         Self {
//             buffers: vec![
//                 Buffer::new(
//                     device_context,
//                     command_pool,
//                     initial_capacity,
//                     usage,
//                     properties,
//                     alloc_dealloc_threshold,
//                     initial_contents,
//                     should_preserve_indices
//                 );
//                 buffer_count
//             ],
//             command_queue,
//             current_frame_index: 0,
//         }
//     }

//     pub fn destroy(&mut self, device: &Device) {
//         self.buffers.iter_mut().for_each(|b| b.destroy(device));
//         self.command_queue.iter_mut().for_each(|q| q.clear());
//         self.command_queue.clear();
//     }

//     pub fn get_current(&self) -> &Buffer<T> {
//         &self.buffers[self.current_frame_index]
//     }

//     pub fn get_current_mut(&mut self) -> &mut Buffer<T> {
//         &mut self.buffers[self.current_frame_index]
//     }

//     /// Tell every frame that this command needs execution
//     pub fn queue_command_duplicated<F>(&mut self, duplicated_command: Vec<Box<F>>)
//     where
//         F: FnOnce(&mut Buffer<T>) + Send + Sync + 'static,
//     {
//         for (i, command) in duplicated_command.into_iter().enumerate() {
//             self.command_queue[i].push(command);
//         }
//     }

//     /// Delay execution of command until the stalest frame is active
//     ///
//     /// Example: If command is queued at [0] [#] [2], it will be executed at [#] [1] [2]
//     pub fn queue_command_delayed<F>(&mut self, command: Box<F>)
//     where
//         F: FnOnce(&mut Buffer<T>) + Send + Sync + 'static,
//     {
//         let max_frames_in_flight = self.command_queue.len();
//         let target_frame =
//             (self.current_frame_index + max_frames_in_flight - 1) % max_frames_in_flight;

//         self.command_queue[target_frame].push(command);
//     }

//     /// Execute commands queued for the current frame in flight
//     pub fn execute_queued_commands(&mut self) {
//         let frame_index = self.current_frame_index;
//         let commands = std::mem::take(&mut self.command_queue[frame_index]);
//         for command in commands {
//             command(&mut self.buffers[frame_index]);
//         }
//         self.current_frame_index = (frame_index + 1) % self.command_queue.len();
//     }
// }
