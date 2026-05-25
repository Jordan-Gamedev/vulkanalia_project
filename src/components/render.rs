#[derive(Clone, Debug, Default)]
pub struct Render {
    pub model_matrix_index: u32, // The model matrix
    pub model_name: String, // The name of the model to render
    pub material: Material, // The material that this entity uses
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool, // Whether this entity is a shadow caster
}

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub sampler_index: u32, // Texture sampler used
    pub albedo_name: String, // Albedo texture used
    pub normal_ao_name: String, // Packed normal map and ambient occlusion map used
    pub metallic_roughness_emissive_name: String, // Packed metallic, roughness, and emissive maps used
}