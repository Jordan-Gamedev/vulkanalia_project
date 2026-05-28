use anyhow::{Result, anyhow};
use glam::{Quat, Vec3};
use std::sync::Arc;
use crate::{ecs::Component, engine::{model_engine::QuantizedModelMatrix}};

#[derive(Clone, Copy, Debug, Default)]
pub struct Transform {
    pub model_matrix_info: u32,
}

impl Component for Transform {
    fn on_add(&mut self, world: &mut crate::ecs::World) {
        unsafe {
            let app = world.app.as_mut().unwrap();
            self.set_model_matrix_index(Arc::make_mut(&mut app.model_engine).create_model_matrix(app.device_context.as_ref().clone().unwrap(), self.is_static()).unwrap());
        }
    }

    fn on_remove(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = world.app.as_mut().unwrap();
            Arc::make_mut(&mut app.model_engine).remove_model_matrix(app.device_context.as_ref().clone().unwrap(), self.get_model_matrix_index(), self.is_static()).unwrap();
        }
    }
}

impl Transform {
    pub fn new() -> Self {
        Transform::default()
    }

    pub fn update_model_matrix(&self, world: &mut crate::ecs::World, position: Vec3, rotation: Quat, scale: Vec3) {
        unsafe {
            let app = world.app.as_mut().unwrap();
            Arc::make_mut(&mut app.model_engine).update_model_matrix(
                self.get_model_matrix_index(),
                position,
                rotation,
                scale,
                self.is_static(),
            ).unwrap();
        }
    }

    pub fn get_quantized_model_matrix(&self, world: &crate::ecs::World) -> Result<QuantizedModelMatrix> {
        unsafe {
            let buffer_contents = if self.is_static() {
                &world.app.as_ref().unwrap().model_engine.static_model_matrices_buffer_contents
            } else {
                &world.app.as_ref().unwrap().model_engine.dyn_model_matrices_buffer_contents
            };

            if let Some(&model_matrix) = buffer_contents.get(self.get_model_matrix_index() as usize) {
                Ok(model_matrix)
            } else {
                Err(anyhow!("Error: failed to get model matrix (index out of bounds)"))
            }
        }
    }

    pub fn get_quantized_model_matrix_mut(&self, world: &crate::ecs::World) -> Result<&mut QuantizedModelMatrix> {
        unsafe {
            let buffer_contents = if self.is_static() {
                &mut Arc::get_mut(&mut world.app.as_mut().unwrap().model_engine).unwrap().static_model_matrices_buffer_contents
            } else {
                &mut Arc::get_mut(&mut world.app.as_mut().unwrap().model_engine).unwrap().dyn_model_matrices_buffer_contents
            };

            if let Some(model_matrix) = buffer_contents.get_mut(self.get_model_matrix_index() as usize) {
                Ok(model_matrix)
            } else {
                Err(anyhow!("Error: failed to get model matrix (index out of bounds)"))
            }
        }
    }

    pub fn is_static(&self) -> bool {
        self.model_matrix_info & 0x80000000 > 0
    }

    /// Mark this transform as static (only works before adding component)
    pub fn set_is_static(&mut self, is_static: bool) {
        self.model_matrix_info &= 0x7FFFFFFF;
        self.model_matrix_info |= (is_static as u32) << 31;
    }

    pub fn get_model_matrix_index(&self) -> u32 {
        self.model_matrix_info & 0x7FFFFFFF
    }

    fn set_model_matrix_index(&mut self, val: u32) {
        self.model_matrix_info = (self.model_matrix_info & 0x80000000) | val;
    }
}