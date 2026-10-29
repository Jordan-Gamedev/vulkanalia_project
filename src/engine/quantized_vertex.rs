use anyhow::{Result, anyhow};
use vulkanalia::prelude::v1_0::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct QuantizedVertex {
    pub position: [f32; 3],
    pub normal: u32,
    pub tangent: u32,
    pub uv: [u16; 2],
    pub bone_ids: [u8; 4],
    pub bone_weights: [u8; 4],
}

impl QuantizedVertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<QuantizedVertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> Result<[vk::VertexInputAttributeDescription; 6]> {
        // Position as XYZ f32 f32 f32
        let position = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        // Normal as XYZ 10bits 10bits 10bits + 2bits to spare
        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::A2R10G10B10_SNORM_PACK32)
            .offset(12)
            .build();

        // Tangent as WXYZ 2bits 10bits 10bits 10bits
        let tangent = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::A2R10G10B10_SNORM_PACK32)
            .offset(16)
            .build();

        // UV as XY f16 f16
        let uv = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R16G16_SFLOAT)
            .offset(20)
            .build();

        // Bone ids as 0-255 0-255 0-255 0-255 8bits 8bits 8bits 8bits
        let bone_ids = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(4)
            .format(vk::Format::R8G8B8A8_UINT)
            .offset(24)
            .build();

        // Max of 4 bone weights as 0-255 0-255 0-255 0-255 8bits 8bits 8bits 8bits
        let bone_weights = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(5)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(28)
            .build();

        Ok([position, bone_ids, normal, tangent, uv, bone_weights])
    }

    pub fn get_supported_vertex_format(
        instance: &Instance,
        physical_device: &vk::PhysicalDevice,
        candidates: &[vk::Format],
        features: vk::FormatFeatureFlags,
    ) -> Result<vk::Format> {
        candidates
            .iter()
            .cloned()
            .find(|f| {
                let properties =
                    unsafe { instance.get_physical_device_format_properties(*physical_device, *f) };
                // For vertex buffers, check buffer features (typically linear tiling)
                properties.buffer_features.contains(features)
            })
            .ok_or_else(|| anyhow!("Failed to find supported vertex attribute format"))
    }
}
