use crate::engine::SamplerContents;
use crate::resources::AssetId;

#[derive(Clone, Copy, Debug, Default)]
pub struct Material {
    pub albedo: AssetId,                      // Albedo texture used
    pub normal_ao: AssetId,                   // Packed normal map and ambient occlusion map used
    pub metallic_roughness_emissive: AssetId, // Packed metallic, roughness, and emissive maps used
    pub sampler_contents: SamplerContents,    // Texture sampler used
}

impl Material {
    pub fn new(
        albedo: AssetId,
        normal_ao: AssetId,
        metallic_roughness_emissive: AssetId,
        sampler_contents: SamplerContents,
    ) -> Self {
        Self {
            albedo,
            normal_ao,
            metallic_roughness_emissive,
            sampler_contents,
        }
    }
}
