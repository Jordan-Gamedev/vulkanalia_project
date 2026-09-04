#![allow(dead_code, unused)]

use crate::components::*;
use crate::engine::*;
use crate::resources::AssetId;
use glam::*;
use vulkanalia::prelude::v1_0::*;

pub trait Prefab<T> {
    fn component_bundle() -> T;
}

pub struct LimpetPrefab;

impl Prefab<(TransformComponent, RenderComponent)> for LimpetPrefab {
    fn component_bundle() -> (TransformComponent, RenderComponent) {
        (
            TransformComponent::new(Vec3::ZERO, Vec3::ONE, Quat::IDENTITY, false),
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
            TransformComponent::new(Vec3::ZERO, Vec3::ONE, Quat::IDENTITY, false),
            RenderComponent::new(
                AssetId::CubeMesh,
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
