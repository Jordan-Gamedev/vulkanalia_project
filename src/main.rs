#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy_app::{Startup, Update};
use bevy_ecs::prelude::*;
use vulkanalia_project::engine::VulkanRenderer;

// Runs exactly once at startup
fn init_spawn(mut _commands: Commands) {}

fn gameplay_update(mut _commands: Commands) {}

fn main() {
    let mut vulkan_renderer = VulkanRenderer::new().unwrap();

    let mut bevy_app = bevy_app::App::new();

    bevy_app
        .add_systems(Startup, init_spawn)
        .add_systems(Update, gameplay_update)
        .set_runner(move |mut bevy_app: bevy_app::App| {
            vulkan_renderer.run(&mut bevy_app).unwrap();
            bevy_app::AppExit::Success
        });

    bevy_app.run();

    // let start = Instant::now();

    // // Create the app
    // let mut app = App::new().unwrap();

    // let duration = start.elapsed();
    // println!("Creating app took: {:?}", duration);

    // let mut bevy_app = bevy_app::App::new();

    // let start = Instant::now();

    // let placement_dist: f32 = 0.05;

    // for z in -5i32..5 {
    //     for x in -5i32..5 {
    //         // Create entity
    //         let entity = app.world.create_entity();

    //         let texture_id = if x % 2 == 0 { AssetId::BlankAlbedoTexture } else { AssetId::CuttlefishAlbedoTexture };
    //         //let verts_id = if x % 2 == 0 { AssetId::CubeVertices } else { AssetId::LimpetVertices };
    //         //let inds_id = if x % 2 == 0 { AssetId::CubeIndices } else { AssetId::LimpetIndices };

    //         // let texture_id = if false { AssetId::BlankAlbedoTexture } else { AssetId::CuttlefishAlbedoTexture };
    //         // let verts_id = if false { AssetId::CubeVertices } else { AssetId::LimpetVertices };
    //         // let inds_id = if false { AssetId::CubeIndices } else { AssetId::LimpetIndices };

    //         let verts_id = match z.abs() % 3 {
    //             0 => {
    //                 AssetId::CubeVertices
    //             },
    //             1 => {
    //                 AssetId::LimpetVertices
    //             },
    //             2 => {
    //                 AssetId::MonkeyVertices
    //             },
    //             _ => {
    //                 AssetId::CubeVertices
    //             }
    //         };

    //         let inds_id = match z.abs() % 3 {
    //             0 => {
    //                 AssetId::CubeIndices
    //             },
    //             1 => {
    //                 AssetId::LimpetIndices
    //             },
    //             2 => {
    //                 AssetId::MonkeyIndices
    //             },
    //             _ => {
    //                 AssetId::CubeIndices
    //             }
    //         };

    //         // Add render component
    //         let mut render_component = Render::new(
    //             verts_id,
    //             inds_id,
    //             Material {
    //                 albedo: texture_id,
    //                 normal_ao: AssetId::None,
    //                 metallic_roughness_emissive: AssetId::None,
    //                 sampler_contents: SamplerContents::new(
    //                     vk::Filter::LINEAR,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerMipmapMode::LINEAR,
    //                 ),
    //             },
    //             true,
    //             true,
    //         );

    //         let position = glam::vec3(x as f32 * placement_dist, 0.0, z as f32 * placement_dist);
    //         let scale = if z % 2 == 0 { glam::vec3(0.01, 0.01, 0.01) } else { glam::vec3(0.01, 0.01, 0.01) };
    //         //let scale = if true { glam::vec3(0.01, 0.01, 0.01) } else { glam::vec3(0.1, 0.1, 0.1) };

    //         render_component.set_is_static(false);
    //         app.world.add_component(entity, render_component.clone());
    //         let render_component = app.world.get_component::<Render>(entity).unwrap().clone();
    //         render_component.set_model_matrix(&mut app.world, position, glam::Quat::IDENTITY, scale);
    //     }
    // }

    // let duration = start.elapsed();
    // println!("Creating entities took: {:?}", duration);

    // // Run the app
    // app.run();
}
