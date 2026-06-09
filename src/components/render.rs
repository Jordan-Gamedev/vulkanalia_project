use crate::{ecs::Component, engine::texture_engine::Material, resources::AlignedAsset};

#[derive(Clone, Debug, Default)]
pub struct Render {
    pub model_vertices: AlignedAsset, // A pointer to the chosen model's vertices
    pub model_indices: AlignedAsset, // A pointer to the chosen model's indices
    pub material: Material, // The material that this entity uses
    pub model_matrix_index: u32, // The model matrix
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool, // Whether this entity is a shadow caster
}

impl Render {
    pub fn new(model_vertices: AlignedAsset, model_indices: AlignedAsset, material: Material, receives_shadows: bool, casts_shadows: bool) -> Self {
        Self {
            model_vertices,
            model_indices,
            material,
            model_matrix_index: 0,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
        }
    }
}

impl Component for Render {
    fn on_add(&mut self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            app.load_texture(self.material.albedo_name.clone(), self.material.sampler_contents).unwrap();
            app.load_model(self.model_vertices, self.model_indices).unwrap();
        }
    }

    fn on_remove(&self, world: &mut crate::ecs::World) {
        unsafe {
            let app = &mut *world.app;
            app.unload_texture(self.material.albedo_name.clone(), self.material.sampler_contents).unwrap();
            app.unload_model(self.model_vertices, self.model_indices).unwrap();
        }
    }
}