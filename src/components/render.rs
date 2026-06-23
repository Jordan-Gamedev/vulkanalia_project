use crate::{components::transform::Transform, ecs::Component, engine::{ModelEngine, command_engine::{PerInstanceData}, texture_engine::Material}, resources::AssetId};

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
    pub fn new(transform: Transform, model_vertices: AssetId, model_indices: AssetId, material: Material, receives_shadows: bool, casts_shadows: bool) -> Self {      
        Self {
            model_vertices,
            model_indices,
            material,
            instance_ptr: std::ptr::null_mut(),
            model_matrix_info: transform.model_matrix_info,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
        }
    }
}

impl Component for Render {
    fn on_add(&mut self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            self.instance_ptr = ModelEngine::create_instance(app, self.model_vertices, self.model_indices, self.material.albedo, self.material.sampler_contents, self.model_matrix_info).unwrap();
        }
    }

    fn on_remove(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;        
            ModelEngine::remove_instance(app, self.model_vertices, self.model_indices, self.material.albedo, self.material.sampler_contents, self.instance_ptr).unwrap();
        }
    }
}