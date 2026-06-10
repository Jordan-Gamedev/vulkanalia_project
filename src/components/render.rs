use crate::{components::transform::Transform, ecs::Component, engine::{command_engine::PerInstanceData, texture_engine::Material}, resources::AssetId};

#[derive(Clone, Debug, Default)]
pub struct Render {
    pub model_vertices: AssetId, // An asset reference to the chosen model's vertices
    pub model_indices: AssetId, // An asset reference to the chosen model's indices
    pub material: Material, // The material that this entity uses
    pub model_matrix_index: u32, // The model matrix
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool, // Whether this entity is a shadow caster
    pub instance_data: PerInstanceData,
}

impl Render {
    pub fn new(transform: Transform, model_vertices: AssetId, model_indices: AssetId, material: Material, receives_shadows: bool, casts_shadows: bool) -> Self {
        let instance_data = PerInstanceData {
            model_matrix_info: transform.model_matrix_info,
            texture_index: 0,
            sampler_index: 0,
            padding: 0
        };
        
        Self {
            model_vertices,
            model_indices,
            material,
            model_matrix_index: 0,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
            instance_data,
        }
    }
}

impl Component for Render {
    fn on_add(&mut self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            app.load_texture(self.material.albedo, self.material.sampler_contents).unwrap();
            app.load_model(self.model_vertices, self.model_indices).unwrap();
            
            // Set bindless texture index
            self.instance_data.texture_index = app
                .texture_engine
                .get_texture_slot_index(self.material.albedo)
                .unwrap_or(0);

            // Set bindless sampler index
            self.instance_data.sampler_index = app
                .texture_engine
                .get_sampler_slot_index(self.material.sampler_contents)
                .unwrap_or(0);
        }
    }

    fn on_remove(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            app.unload_texture(self.material.albedo, self.material.sampler_contents).unwrap();
            app.unload_model(self.model_vertices, self.model_indices).unwrap();
        }
    }
}