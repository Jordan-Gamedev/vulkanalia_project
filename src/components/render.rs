use anyhow::{Result, anyhow};
use glam::{Quat, Vec3};
use std::sync::Arc;

use crate::{ecs::Component, engine::{ModelEngine, command_engine::PerInstanceData, model_engine::QuantizedModelMatrix, texture_engine::Material}, resources::AssetId};

#[derive(Clone, Debug, Default)]
pub struct Render {
    pub model_vertices: AssetId, // An asset reference to the chosen model's vertices
    pub model_indices: AssetId, // An asset reference to the chosen model's indices
    pub material: Material, // The material that this entity uses
    pub instance_ptr: *mut PerInstanceData, // Location of render instance in instance buffer
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
            instance_ptr: std::ptr::null_mut(),
            model_matrix_info: u32::MAX,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
        }
    }

    pub fn set_model_matrix(&self, world: &mut crate::ecs::World, position: Vec3, rotation: Quat, scale: Vec3, save_changes: bool) {
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

            if save_changes {
                model_engine.save_model_matrix_changes(
                    app.device_context.as_ref().clone().unwrap().device,
                    self.get_model_matrix_index(),
                    self.is_static(),
                );
            }
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

    pub fn save_transform_changes(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = world.app.as_mut().unwrap();
            Arc::make_mut(&mut app.model_engine).save_model_matrix_changes(
                app.device_context.as_ref().clone().unwrap().device,
                self.get_model_matrix_index(),
                self.is_static(),
            );
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
            self.instance_ptr = ModelEngine::create_instance(app, self.model_vertices, self.model_indices, self.material.albedo, self.material.sampler_contents, self.model_matrix_info).unwrap();

            let query = world.query_opt::<Render>().unwrap();
            for render in query {
                for i in 0..app.command_engine.instance_capacity {
                    let instance_ptr = app.command_engine.instance_buffer_mapped.add(i);
                    if render.model_matrix_info == instance_ptr.read().model_matrix_info {
                        render.instance_ptr = instance_ptr;
                        break;
                    }
                }
            }
        }
    }

    fn on_remove(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            println!("model matrix: {}, instance matrix: {}", self.model_matrix_info, self.instance_ptr.read().model_matrix_info);
            ModelEngine::remove_instance(app, self.model_vertices, self.model_indices, self.material.albedo, self.material.sampler_contents, self.instance_ptr).unwrap();
            ModelEngine::remove_model_matrix(app, self.get_model_matrix_index(), self.is_static()).unwrap();
        
            let query = world.query_opt::<Render>().unwrap();
            for render in query {
                for i in 0..app.command_engine.instance_capacity {
                    let instance_ptr = app.command_engine.instance_buffer_mapped.add(i);
                    if render.model_matrix_info == instance_ptr.read().model_matrix_info {
                        render.instance_ptr = instance_ptr;
                        break;
                    }
                }
            }
        }
    }
}