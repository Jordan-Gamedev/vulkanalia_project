use crate::engine::SamplerContents;
use crate::resources::AssetId;

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub albedo: AssetId,                      // Albedo texture used
    pub normal_ao: AssetId,                   // Packed normal map and ambient occlusion map used
    pub metallic_roughness_emissive: AssetId, // Packed metallic, roughness, and emissive maps used
    pub sampler_contents: SamplerContents,    // Texture sampler used
}
