#[derive(Clone, Debug)]
pub struct Render {
    pub ubo_index: u32, // The model matrix
    pub vertex_offset: u32, // The vertex buffer offset
    pub index_offset: u32, // The index buffer offset
    pub material: Material, // The material that this entity uses
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool, // Whether this entity is a shadow caster
}

#[derive(Clone, Debug)]
pub struct Material {
    pub sampler_index: u32, // Texture sampler used (u32::MAX = None)
    pub albedo_index: u32, // Albedo texture used (u32::MAX = None)
    pub normal_ao_index: u32, // Normal map and ambient occlusion map used (u32::MAX = None)
    pub metallic_roughness_emissive_index: u32, // Packed metallic, roughness, and emissive maps used (u32::MAX = None)
}