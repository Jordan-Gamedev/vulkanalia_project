use crate::engine::SamplerContents;
use crate::engine::SamplerUsage;
use crate::engine::TextureUsage;
use crate::resources::AssetId;
use std::collections::HashMap;

#[derive(Default)]
pub struct TextureHandle {
    pub loaded_textures: HashMap<AssetId, TextureUsage>,
    pub available_texture_slots: Vec<u32>,
    pub samplers: HashMap<SamplerContents, SamplerUsage>,
    pub available_sampler_slots: Vec<u32>,
}
