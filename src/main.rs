#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy_app::{Startup, Update};
use bevy_ecs::prelude::*;
use std::sync::Arc;
use vulkanalia::prelude::v1_0::*;
use vulkanalia_project::components::RenderComponent;
use vulkanalia_project::engine::Material;
use vulkanalia_project::engine::SamplerContents;
use vulkanalia_project::engine::VulkanRenderer;
use vulkanalia_project::resources::*;
use winit::event::{Event, WindowEvent};

// Runs exactly once at startup
fn init_spawn(mut commands: Commands) {
    let sampler_contents = SamplerContents::new(
        vk::Filter::LINEAR,
        vk::SamplerAddressMode::REPEAT,
        vk::SamplerAddressMode::REPEAT,
        vk::SamplerAddressMode::REPEAT,
        vk::SamplerMipmapMode::LINEAR,
    );

    let material = Material {
        albedo: AssetId::BlankAlbedoTexture,
        normal_ao: AssetId::None,
        metallic_roughness_emissive: AssetId::None,
        sampler_contents: sampler_contents,
    };
    commands.spawn(RenderComponent::new(
        AssetId::CubeVertices,
        AssetId::CubeIndices,
        material.clone(),
        false,
        false,
    ));

    // commands.spawn(RenderComponent::new(
    //     AssetId::LimpetVertices,
    //     AssetId::LimpetIndices,
    //     material,
    //     false,
    //     false,
    // ));
}

fn gameplay_update(mut _commands: Commands) {
    //println!("UPDATE");
}

fn main() {
    // let mut bevy_app = bevy_app::App::new();
    // bevy_app.insert_resource(VulkanRenderer::new().unwrap());

    // bevy_app
    //     .add_systems(Startup, init_spawn)
    //     .add_systems(Update, gameplay_update);

    // let mut vulkan_renderer = bevy_app
    //     .world_mut()
    //     .get_resource_mut::<VulkanRenderer>()
    //     .unwrap();

    // vulkan_renderer.run(&mut bevy_app).unwrap();
    let mut bevy_app = bevy_app::App::new();

    // 1. Initialize renderer
    let mut renderer = VulkanRenderer::new().unwrap();

    // 2. Take the event loop out completely so it doesn't tie up 'self'
    let event_loop = Arc::into_inner(
        renderer
            .present_handle
            .window_handle
            .event_loop
            .take()
            .unwrap(),
    )
    .unwrap();

    // 3. Put the renderer into Bevy
    bevy_app.insert_resource(renderer);

    bevy_app
        .add_systems(Startup, init_spawn)
        .add_systems(Update, gameplay_update);

    // Set a custom runner that drives the event loop and satisfies the return type
    bevy_app.set_runner(move |mut app| {
        event_loop
            .run(move |event, elwt| {
                match event {
                    Event::AboutToWait => {
                        if let Some(renderer) = app.world().get_resource::<VulkanRenderer>() {
                            renderer
                                .present_handle
                                .window_handle
                                .window
                                .request_redraw();
                        }
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::RedrawRequested if !elwt.exiting() => {
                            // 1. Run systems
                            app.update();

                            // 2. Render
                            let mut renderer = app
                                .world_mut()
                                .get_resource_mut::<VulkanRenderer>()
                                .unwrap();
                            renderer.render().unwrap();
                        }

                        WindowEvent::Resized(_size) => {
                            let mut renderer = app
                                .world_mut()
                                .get_resource_mut::<VulkanRenderer>()
                                .unwrap();
                            renderer.present_handle.window_handle.is_resized = true;
                        }

                        WindowEvent::CloseRequested => {
                            let mut renderer = app
                                .world_mut()
                                .get_resource_mut::<VulkanRenderer>()
                                .unwrap();
                            renderer
                                .present_handle
                                .window_handle
                                .window
                                .set_visible(false);
                            renderer.destroy();
                            elwt.exit();
                        }
                        _ => {}
                    },
                    _ => {}
                }
            })
            .unwrap();

        bevy_app::AppExit::Success
    });

    bevy_app.run();
}

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
//                     vk::Filter::LINEAR,
//                     vk::SamplerAddressMode::REPEAT,
//                     vk::SamplerAddressMode::REPEAT,
//                     vk::SamplerAddressMode::REPEAT,
//                     vk::SamplerMipmapMode::LINEAR,
//                 )
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
