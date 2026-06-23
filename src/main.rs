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

    for x in -128..128 {
        for z in -128..128 {
            // Create entity
            let entity = app.world.create_entity();

            // Add transform component
            let mut transform_component = Transform::new();
            if true {
                transform_component.set_is_static(true);
            }
            app.world.add_component(entity, transform_component);

            // let texture_id = if x % 2 == 0 { AssetId::BlankAlbedoTexture } else { AssetId::CuttlefishAlbedoTexture };
            let verts_id = if x % 2 == 0 { AssetId::CubeVertices } else { AssetId::LimpetVertices };
            let inds_id = if x % 2 == 0 { AssetId::CubeIndices } else { AssetId::LimpetIndices };

            let texture_id = if false { AssetId::BlankAlbedoTexture } else { AssetId::CuttlefishAlbedoTexture };
            // let verts_id = if true { AssetId::CubeVertices } else { AssetId::LimpetVertices };
            // let inds_id = if true { AssetId::CubeIndices } else { AssetId::LimpetIndices };

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

    // {
    //     // Create entity with a renderer
    //     let entity = app.world.create_entity();
    //     let render_component = Render::new(
    //         "assets/models_compressed/Limpet".to_string(),
    //         Material {
    //             albedo_name: "assets/textures/cuttlefish_albedo".to_string(),
    //             normal_ao_name: String::new(),
    //             metallic_roughness_emissive_name: String::new(),
    //             sampler_contents: SamplerContents::new(
    //                 vk::Filter::LINEAR,
    //                 vk::SamplerAddressMode::REPEAT,
    //                 vk::SamplerAddressMode::REPEAT,
    //                 vk::SamplerAddressMode::REPEAT,
    //                 vk::SamplerMipmapMode::LINEAR,
    //             ),
    //         },
    //         true,
    //         true,
    //     );
    //     let mut transform_component = Transform::new();
    //     transform_component.set_is_static(true);
        
    //     app.world.add_component(entity, render_component);
    //     app.world.add_component(entity, transform_component);

    //     let transform_component = *app.world.get_component::<Transform>(entity).unwrap();
    //     transform_component.set_model_matrix(&mut app.world, glam::vec3(-2.0, 0.0, 0.0), glam::Quat::IDENTITY, glam::Vec3::ONE);
    // }

    // {
    //     // Create entity with a renderer
    //     let entity = app.world.create_entity();
    //     let render_component = Render::new(
    //         "assets/models_compressed/Cube".to_string(),
    //         Material {
    //             albedo_name: "assets/textures/blank_albedo".to_string(),
    //             normal_ao_name: String::new(),
    //             metallic_roughness_emissive_name: String::new(),
    //             sampler_contents: SamplerContents::new(
    //                 vk::Filter::LINEAR,
    //                 vk::SamplerAddressMode::REPEAT,
    //                 vk::SamplerAddressMode::REPEAT,
    //                 vk::SamplerAddressMode::REPEAT,
    //                 vk::SamplerMipmapMode::LINEAR,
    //             ),
    //         },
    //         true,
    //         true,
    //     );
    //     let mut transform_component = Transform::new();
    //     transform_component.set_is_static(false);
        
    //     app.world.add_component(entity, render_component);
    //     app.world.add_component(entity, transform_component);

    //     let transform_component = *app.world.get_component::<Transform>(entity).unwrap();
    //     transform_component.set_model_matrix(&mut app.world, glam::vec3(0.0, 0.0, 0.0), glam::Quat::IDENTITY, glam::Vec3::ONE);
    // }

    // Run the app
    app.run();
}