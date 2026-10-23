use crate::components::TransformComponent;
use bevy_ecs::component::Component;
use glam::Vec3;

#[derive(Component)]
#[require(TransformComponent::default())]
pub struct CameraComponent {
    pub near_plane: f32,
    pub far_plane: f32,
    pub fov: f32,
    pub rotation_eulers: Vec3,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            near_plane: 0.1,
            far_plane: 1000.0,
            fov: 45.0,
            rotation_eulers: Vec3::ZERO,
        }
    }
}

impl CameraComponent {
    pub fn new(near_plane: f32, far_plane: f32, fov: f32) -> Self {
        Self {
            near_plane,
            far_plane,
            fov,
            rotation_eulers: Vec3::ZERO,
        }
    }
}
