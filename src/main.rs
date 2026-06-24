#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Instant;

use vulkanalia::prelude::v1_0::*;
use vulkanalia_project::components::render::Render;
use vulkanalia_project::components::transform::Transform;
use vulkanalia_project::engine::App;
use vulkanalia_project::engine::texture_engine::{Material, SamplerContents};
use vulkanalia_project::resources::AssetId;


fn main() {

    let start = Instant::now();

    // Create the app
    let mut app = App::new().unwrap();

    let duration = start.elapsed();
    println!("Creating app took: {:?}", duration);


    let start = Instant::now();

    let placement_dist: f32 = 0.1;

    for x in -2i32..4 {
        for z in -2..2 {
            // Create entity
            let entity = app.world.create_entity();

            // Add transform component
            let mut transform_component = Transform::new();
            if false {
                transform_component.set_is_static(true);
            }
            app.world.add_component(entity, transform_component);

            let texture_id = if x % 2 == 0 { AssetId::BlankAlbedoTexture } else { AssetId::CuttlefishAlbedoTexture };
            //let verts_id = if x % 2 == 0 { AssetId::CubeVertices } else { AssetId::LimpetVertices };
            //let inds_id = if x % 2 == 0 { AssetId::CubeIndices } else { AssetId::LimpetIndices };

            // let texture_id = if false { AssetId::BlankAlbedoTexture } else { AssetId::CuttlefishAlbedoTexture };
            // let verts_id = if false { AssetId::CubeVertices } else { AssetId::LimpetVertices };
            // let inds_id = if false { AssetId::CubeIndices } else { AssetId::LimpetIndices };

            let verts_id = match x.abs() % 3 {
                0 => {
                    AssetId::CubeVertices
                },
                1 => {
                    AssetId::LimpetVertices
                },
                2 => {
                    AssetId::MonkeyVertices
                },
                _ => {
                    AssetId::CubeVertices
                }
            };

            let inds_id = match x.abs() % 3 {
                0 => {
                    AssetId::CubeIndices
                },
                1 => {
                    AssetId::LimpetIndices
                },
                2 => {
                    AssetId::MonkeyIndices
                },
                _ => {
                    AssetId::CubeIndices
                }
            };

            // Add render component
            let render_component = Render::new(
                *app.world.get_component::<Transform>(entity).unwrap(),
                verts_id,
                inds_id,
                Material {
                    albedo: texture_id,
                    normal_ao: AssetId::None,
                    metallic_roughness_emissive: AssetId::None,
                    sampler_contents: SamplerContents::new(
                        vk::Filter::LINEAR,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerAddressMode::REPEAT,
                        vk::SamplerMipmapMode::LINEAR,
                    ),
                },
                true,
                true,
            );
            app.world.add_component(entity, render_component);
    
            let position = glam::vec3(x as f32 * placement_dist, 0.0, z as f32 * placement_dist);
            let scale = if x % 2 == 0 { glam::vec3(0.01, 0.01, 0.01) } else { glam::vec3(0.01, 0.01, 0.01) };
            //let scale = if true { glam::vec3(0.01, 0.01, 0.01) } else { glam::vec3(0.1, 0.1, 0.1) };

            let transform_component = *app.world.get_component::<Transform>(entity).unwrap();
            transform_component.set_model_matrix(&mut app.world, position, glam::Quat::IDENTITY, scale);
        }
    }

    let duration = start.elapsed();
    println!("Creating entities took: {:?}", duration);

    // Run the app
    app.run();
}