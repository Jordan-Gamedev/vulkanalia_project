use crate::engine::Texture;

#[derive(Clone, Copy, Debug)]
pub struct TextureUsage {
    pub texture: Texture,
    pub slot_index: u32,
    pub usage_count: u32,
    pub lifetime: u32,
}
