use bevy_ecs::component::Component;
use glam::Quat;
use glam::Vec3;

use crate::engine::QuantizedModelMatrix;

#[derive(Component, Clone, Copy, Debug)]
pub struct TransformComponent {
    pub position: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
    pub model_matrix_index: u32,
    pub is_static: bool,
}

impl Default for TransformComponent {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            scale: Vec3::ONE,
            rotation: Quat::IDENTITY,
            model_matrix_index: u32::MAX,
            is_static: false,
        }
    }
}

impl TransformComponent {
    pub fn new(position: Vec3, scale: Vec3, rotation: Quat, is_static: bool) -> Self {
        Self {
            position,
            scale,
            rotation,
            model_matrix_index: u32::MAX,
            is_static,
        }
    }

    pub fn to_quantized_matrix(&self) -> QuantizedModelMatrix {
        let rotation_i16: [i16; 4] = [
            (self.rotation.x * i16::MAX as f32) as i16,
            (self.rotation.y * i16::MAX as f32) as i16,
            (self.rotation.z * i16::MAX as f32) as i16,
            (self.rotation.w * i16::MAX as f32) as i16,
        ];

        QuantizedModelMatrix {
            position: self.position.to_array(),
            scale: self.scale.to_array(),
            rotation: rotation_i16,
        }
    }

    pub fn has_render(&self) -> bool {
        self.model_matrix_index != u32::MAX
    }
}
