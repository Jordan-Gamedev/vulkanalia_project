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
    // pub const fn from_slice(slice: &[u8; 16]) -> Self {
    //     let position = [
    //         u16::from_le_bytes([slice[0], slice[1]]),
    //         u16::from_le_bytes([slice[2], slice[3]]),
    //         u16::from_le_bytes([slice[4], slice[5]]),
    //     ];
    //     let color = [slice[6], slice[7], slice[8]];
    //     let normal = [slice[9] as i8, slice[10] as i8, slice[11] as i8];
    //     let uv = [
    //         u16::from_le_bytes([slice[12], slice[13]]),
    //         u16::from_le_bytes([slice[14], slice[15]]),
    //     ];

    //     Self {
    //         position,
    //         color,
    //         normal,
    //         uv,
    //     }
    // }

    // pub fn to_vertex(&self) -> Vertex {
    //     let position: Vec3 = vec3(
    //         meshopt::dequantize_half(self.position[0]),
    //         meshopt::dequantize_half(self.position[1]),
    //         meshopt::dequantize_half(self.position[2]),
    //     );

    //     let color: Vec3 = vec3(
    //         self.color[0] as f32 / u8::MAX as f32,
    //         self.color[1] as f32 / u8::MAX as f32,
    //         self.color[2] as f32 / u8::MAX as f32,
    //     );

    //     let normal: Vec3 = vec3(
    //         self.normal[0] as f32 / i8::MAX as f32,
    //         self.normal[1] as f32 / i8::MAX as f32,
    //         self.normal[2] as f32 / i8::MAX as f32,
    //     );

    //     let uv: Vec2 = vec2(
    //         self.uv[0] as f32 / u16::MAX as f32,
    //         self.uv[1] as f32 / u16::MAX as f32,
    //     );

    //     Vertex {
    //         pos: position,
    //         color: color,
    //         normal: normal,
    //         uv: uv,
    //     }
    // }

    // pub fn attribute_descriptions(
    //     instance: &Instance,
    //     physical_device: &vk::PhysicalDevice,
    // ) -> Result<[vk::VertexInputAttributeDescription; 4]> {
    //     // Try preferred format, fall back if not supported
    //     // Vertex attributes typically support FORMAT_VERTEX_BUFFER feature
    //     let features = vk::FormatFeatureFlags::VERTEX_BUFFER;

    //     // Position: Try R16G16B16_SFLOAT, fall back to R16G16B16A16_SFLOAT
    //     let pos_format = QuantizedVertex::get_supported_vertex_format(
    //         instance,
    //         physical_device,
    //         &[
    //             vk::Format::R16G16B16_SFLOAT,
    //             vk::Format::R16G16B16A16_SFLOAT,
    //         ],
    //         features,
    //     )?;
    //     let pos = vk::VertexInputAttributeDescription::builder()
    //         .binding(0)
    //         .location(0)
    //         .format(pos_format)
    //         .offset(0)
    //         .build();

    //     // Color: Try R8G8B8_UNORM, fall back to R8G8B8A8_UNORM
    //     let color_format = QuantizedVertex::get_supported_vertex_format(
    //         instance,
    //         physical_device,
    //         &[vk::Format::R8G8B8_UNORM, vk::Format::R8G8B8A8_UNORM],
    //         features,
    //     )?;
    //     let color = vk::VertexInputAttributeDescription::builder()
    //         .binding(0)
    //         .location(1)
    //         .format(color_format)
    //         .offset(size_of::<[u16; 3]>() as u32)
    //         .build();

    //     // Normal: Try R8G8B8_SNORM, fall back to R8G8B8A8_SNORM
    //     let normal_format = QuantizedVertex::get_supported_vertex_format(
    //         instance,
    //         physical_device,
    //         &[vk::Format::R8G8B8_SNORM, vk::Format::R8G8B8A8_SNORM],
    //         features,
    //     )?;
    //     let normal = vk::VertexInputAttributeDescription::builder()
    //         .binding(0)
    //         .location(2)
    //         .format(normal_format)
    //         .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>()) as u32)
    //         .build();

    //     // UV: Use R16G16_UNORM
    //     let uv = vk::VertexInputAttributeDescription::builder()
    //         .binding(0)
    //         .location(3)
    //         .format(vk::Format::R16G16_UNORM)
    //         .offset((size_of::<[u16; 3]>() + size_of::<[u8; 3]>() + size_of::<[i8; 3]>()) as u32)
    //         .build();

    //     Ok([pos, color, normal, uv])
    // }

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
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(0)
            .build();

        // Normal as XYZ 10bits 10bits 10bits + 2bits to spare
        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::A2R10G10B10_SNORM_PACK32)
            .offset(size_of::<[f32; 4]>() as u32)
            .build();

        // Tangent as WXYZ 2bits 10bits 10bits 10bits
        let tangent = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::A2R10G10B10_SNORM_PACK32)
            .offset(size_of::<[f32; 4]>() as u32 + size_of::<u32>() as u32)
            .build();

        // UV as XY f16 f16
        let uv = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R16G16_SFLOAT)
            .offset(
                size_of::<[f32; 4]>() as u32 + size_of::<u32>() as u32 + size_of::<u32>() as u32,
            )
            .build();

        // Bone ids as 0-255 0-255 0-255 0-255 8bits 8bits 8bits 8bits
        let bone_ids = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(4)
            .format(vk::Format::R8G8B8A8_UINT)
            .build();

        // Max of 4 bone weights as 0-255 0-255 0-255 0-255 8bits 8bits 8bits 8bits
        let bone_weights = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(5)
            .format(vk::Format::R8G8B8A8_UNORM)
            .offset(
                size_of::<[f32; 4]>() as u32
                    + size_of::<u32>() as u32
                    + size_of::<u32>() as u32
                    + size_of::<[u16; 2]>() as u32,
            )
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
