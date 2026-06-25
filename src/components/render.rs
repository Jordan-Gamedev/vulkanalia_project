use anyhow::{Result};
use glam::{Quat, Vec3};
use std::sync::Arc;

use crate::{ecs::Component, engine::{ModelEngine, model_engine::QuantizedModelMatrix, texture_engine::Material}, resources::AssetId};

#[derive(Clone, Debug, Default)]
pub struct Render {
    pub model_vertices: AssetId, // An asset reference to the chosen model's vertices
    pub model_indices: AssetId, // An asset reference to the chosen model's indices
    pub material: Material, // The material that this entity uses
    pub model_matrix_info: u32, // The model matrix
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool, // Whether this entity is a shadow caster
}

impl Render {
    pub fn new(model_vertices: AssetId, model_indices: AssetId, material: Material, receives_shadows: bool, casts_shadows: bool) -> Self {      
        Self {
            model_vertices,
            model_indices,
            material,
            model_matrix_info: u32::MAX,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
        }
    }

    pub fn set_model_matrix(&self, world: &mut crate::ecs::World, position: Vec3, rotation: Quat, scale: Vec3) {
        unsafe {
            let app = world.app.as_mut().unwrap();
            let model_engine = Arc::make_mut(&mut app.model_engine);

            model_engine.set_model_matrix(
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
            let app = world.app.as_mut().unwrap();
            let model_matrix = app.model_engine.get_model_matrix(self.get_model_matrix_index(), self.is_static());
            Ok(model_matrix)
        }
    }

    pub fn get_quantized_model_matrix_mut(&self, world: &crate::ecs::World) -> Result<&mut QuantizedModelMatrix> {
        unsafe {
            let app = world.app.as_mut().unwrap();
            let model_matrix = app.model_engine.get_model_matrix_mut(self.get_model_matrix_index(), self.is_static());
            Ok(model_matrix)
        }
    }

    pub fn is_static(&self) -> bool {
        self.model_matrix_info & 0x80000000 > 0
    }

    /// Mark this render transform as static (only works before adding component)
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

impl Component for Render {
    fn on_add(&mut self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            self.set_model_matrix_index(ModelEngine::create_model_matrix(app, self.is_static()).unwrap());
            ModelEngine::create_instance(app, self.model_vertices, self.model_indices, self.material.albedo, self.material.sampler_contents, self.model_matrix_info).unwrap();
        }
    }

    fn on_remove(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            for i in 0..app.command_engine.instance_capacity {
                if app.command_engine.instance_buffer_mapped.add(i).read().model_matrix_info == self.model_matrix_info {
                    ModelEngine::remove_instance(app, self.model_vertices, self.model_indices, self.material.albedo, self.material.sampler_contents, app.command_engine.instance_buffer_mapped.add(i)).unwrap();
                    break;
                }
            }
            ModelEngine::remove_model_matrix(app, self.get_model_matrix_index(), self.is_static()).unwrap();
        }
    }
}