#![allow(dead_code, unused)]

use crate::components::*;
use crate::engine::*;
use crate::resources::AssetId;
use glam::*;
use std::f32::consts::PI;
use vulkanalia::prelude::v1_0::*;

const DEG_TO_RAD: f32 = PI / 180.0;

pub trait Prefab<T> {
    fn component_bundle() -> T;
}

pub struct LimpetPrefab;

impl Prefab<(TransformComponent, RenderComponent)> for LimpetPrefab {
    fn component_bundle() -> (TransformComponent, RenderComponent) {
        (
            TransformComponent::new(
                Vec3::new(0.0, 0.0, 10.0),
                Vec3::new(1.0, 1.0, 1.0),
                Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 45.0 * DEG_TO_RAD),
            ),
            RenderComponent::new(
                AssetId::LimpetMesh,
                Material::new(
                    AssetId::CuttlefishAlbedoTexture,
                    AssetId::None,
                    AssetId::None,
                    SamplerContents::new(
                        vk::Filter::LINEAR,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerMipmapMode::LINEAR,
                    ),
                ),
                true,
                true,
            ),
        )
    }
}

pub struct CubePrefab;

impl Prefab<(TransformComponent, RenderComponent)> for CubePrefab {
    fn component_bundle() -> (TransformComponent, RenderComponent) {
        (
            TransformComponent::new(
                Vec3::new(1.0, 3.0, 10.0),
                Vec3::new(0.5, 0.5, 0.5),
                Quat::IDENTITY,
            ),
            RenderComponent::new(
                AssetId::CubeMesh,
                Material::new(
                    AssetId::BlankAlbedoTexture,
                    AssetId::None,
                    AssetId::None,
                    SamplerContents::new(
                        vk::Filter::LINEAR,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerMipmapMode::LINEAR,
                    ),
                ),
                true,
                true,
            ),
        )
    }
}
