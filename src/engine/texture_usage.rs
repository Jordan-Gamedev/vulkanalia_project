use crate::engine::Texture;

#[derive(Clone, Copy, Debug, Default)]
pub struct TextureUsage {
    pub texture: Texture,
    pub slot_index: u32,
    pub instance_count: u32,
}
