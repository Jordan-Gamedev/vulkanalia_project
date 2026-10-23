use bevy_ecs::system::Single;
use std::f32::consts::PI;

use crate::components::CameraComponent;
use crate::components::TransformComponent;
use crate::engine::UniformBufferObject;
use crate::engine::vulkan_renderer::VULKAN_RENDERER_SINGLETON;

const DEG_TO_RAD: f32 = PI / 180.0;

pub fn camera_render_system(camera_components: Single<(&CameraComponent, &TransformComponent)>) {
    let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
    let (cam, transform) = camera_components.into_inner();

    // View + Projection

    let forward = transform.forward();
    let up = transform.up();
    let view = glam::camera::rh::view::look_to_mat4(transform.position, forward, up);

    let proj = glam::camera::rh::proj::vulkan::perspective(
        cam.fov * DEG_TO_RAD,
        renderer.present_handle.swapchain_handle.extent.width as f32
            / renderer.present_handle.swapchain_handle.extent.height as f32,
        cam.near_plane,
        cam.far_plane,
    );

    let ubo = UniformBufferObject { view, proj };

    // Copy

    unsafe {
        std::ptr::copy_nonoverlapping(
            &ubo,
            renderer
                .model_handle
                .uniform_buffers
                .get_current_mut()
                .mapped
                .cast_mut(),
            1,
        );
    }
}
