use vulkanalia::prelude::v1_0::*;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct SamplerContents {
    pub filter: vk::Filter,
    pub address_mode_u: vk::SamplerAddressMode,
    pub address_mode_v: vk::SamplerAddressMode,
    pub address_mode_w: vk::SamplerAddressMode,
    pub mipmap_mode: vk::SamplerMipmapMode,
    pub mipmap_levels: u32,
}

impl SamplerContents {
    pub fn new(
        filter: vk::Filter,
        address_mode_u: vk::SamplerAddressMode,
        address_mode_v: vk::SamplerAddressMode,
        address_mode_w: vk::SamplerAddressMode,
        mipmap_mode: vk::SamplerMipmapMode,
    ) -> Self {
        Self {
            filter,
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mipmap_mode,
            mipmap_levels: 0,
        }
    }
}
