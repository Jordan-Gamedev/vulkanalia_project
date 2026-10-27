#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused_imports)]
use bevy_app::{Startup, Update};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_input::gamepad::Gamepad;
use bevy_input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy_input::{ButtonInput, InputPlugin};
use bevy_prng::WyRand;
use bevy_rand::global::GlobalRng;
use bevy_rand::prelude::EntropyPlugin;
use bevy_time::{Time, TimeUpdateStrategy};
use glam::{Quat, Vec3};
use rand::{Rng, RngExt};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Instant;
use vulkanalia::prelude::v1_0::*;
use vulkanalia_project::components::*;
use vulkanalia_project::engine::Material;
use vulkanalia_project::engine::SamplerContents;
use vulkanalia_project::engine::VulkanRenderer;
use vulkanalia_project::engine::vulkan_renderer::VULKAN_RENDERER_SINGLETON;
use vulkanalia_project::input_manager::InputManager;
use vulkanalia_project::resources::*;
use vulkanalia_project::systems::camera_move_system::camera_move_system;
use vulkanalia_project::systems::camera_render_system::camera_render_system;
use vulkanalia_project::{input_manager, prefabs::*};
use winit::dpi::PhysicalPosition;
use winit::event::DeviceEvent::MouseMotion;
use winit::event::WindowEvent::KeyboardInput;
use winit::event::{DeviceEvent, ElementState, MouseButton};
use winit::event::{Event, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::CursorGrabMode;

// Runs exactly once at startup
fn init_spawn(mut _rng: Single<&mut WyRand, With<GlobalRng>>, mut commands: Commands) {
    commands.spawn(CameraComponent::default());
    commands.spawn(CubePrefab::component_bundle());
    commands.spawn(LimpetPrefab::component_bundle());

    // let num_cubes = rng.random_range(32..=32);
    // for _ in 0..num_cubes {
    //     let spawn_pos: Vec3 = Vec3::new(
    //         rng.as_mut().random_range(-50.0..=50.0) as f32,
    //         rng.as_mut().random_range(-50.0..=50.0) as f32,
    //         rng.as_mut().random_range(-50.0..=50.0) as f32,
    //     );

    //     let random_texture: AssetId =
    //         AssetId::from_repr(rng.random_range(START_ENUM_T..=END_ENUM_T) as u16).unwrap();

    //     let random_size = rng.random_range(0.05..=2.0);

    //     commands.spawn((
    //         TransformComponent::new(
    //             spawn_pos,
    //             Vec3::new(random_size, random_size, random_size),
    //             Quat::from_euler(
    //                 glam::EulerRot::YXZ,
    //                 rng.random_range(-180.0..180.0),
    //                 rng.random_range(-180.0..180.0),
    //                 rng.random_range(-180.0..180.0),
    //             ),
    //         ),
    //         RenderComponent::new(
    //             AssetId::CubeMesh,
    //             Material::new(
    //                 random_texture,
    //                 AssetId::None,
    //                 AssetId::None,
    //                 SamplerContents::new(
    //                     vk::Filter::LINEAR,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerMipmapMode::LINEAR,
    //                 ),
    //             ),
    //             true,
    //             true,
    //         ),
    //     ));
    // }

    // let num_limpets = rng.random_range(32..=32);
    // for _ in 0..num_limpets {
    //     let spawn_pos: Vec3 = Vec3::new(
    //         rng.as_mut().random_range(-50.0..=50.0) as f32,
    //         rng.as_mut().random_range(-50.0..=50.0) as f32,
    //         rng.as_mut().random_range(-50.0..=50.0) as f32,
    //     );

    //     let random_texture: AssetId =
    //         AssetId::from_repr(rng.random_range(START_ENUM_T..=END_ENUM_T) as u16).unwrap();

    //     let random_size = rng.random_range(0.05..=2.0);

    //     commands.spawn((
    //         TransformComponent::new(
    //             spawn_pos,
    //             Vec3::new(random_size, random_size, random_size),
    //             Quat::from_euler(
    //                 glam::EulerRot::YXZ,
    //                 rng.random_range(-180.0..180.0),
    //                 rng.random_range(-180.0..180.0),
    //                 rng.random_range(-180.0..180.0),
    //             ),
    //         ),
    //         RenderComponent::new(
    //             AssetId::LimpetMesh,
    //             Material::new(
    //                 random_texture,
    //                 AssetId::None,
    //                 AssetId::None,
    //                 SamplerContents::new(
    //                     vk::Filter::LINEAR,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerAddressMode::REPEAT,
    //                     vk::SamplerMipmapMode::LINEAR,
    //                 ),
    //             ),
    //             true,
    //             true,
    //         ),
    //     ));
    // }
}

fn gameplay_update(mut _commands: Commands) {
    //println!("Gameplay UPDATE");
}

#[derive(Resource)]
struct TempStruct {
    pub cube_count: u64,
    pub limpet_count: u64,
}

fn temp_spawn_test(
    mut temp_struct: ResMut<TempStruct>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    render_query: Query<(Entity, &RenderComponent)>,
    mut commands: Commands,
) {
    return;
    if keyboard.pressed(KeyCode::Space) {
        let spawn_pos: Vec3 = Vec3::new(
            rng.as_mut().random_range(-50.0..=50.0) as f32,
            rng.as_mut().random_range(-50.0..=50.0) as f32,
            rng.as_mut().random_range(-50.0..=50.0) as f32,
        );

        let random_texture: AssetId =
            AssetId::from_repr(rng.random_range(START_ENUM_T..=END_ENUM_T) as u16).unwrap();

        let random_size = rng.random_range(0.05..=2.0);

        commands.spawn((
            TransformComponent::new(
                spawn_pos,
                Vec3::new(random_size, random_size, random_size),
                Quat::from_euler(
                    glam::EulerRot::YXZ,
                    rng.random_range(-180.0..180.0),
                    rng.random_range(-180.0..180.0),
                    rng.random_range(-180.0..180.0),
                ),
            ),
            RenderComponent::new(
                AssetId::CubeMesh,
                Material::new(
                    random_texture,
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
        ));

        temp_struct.cube_count += 1;
        println!(
            "cubes: {} limpets: {} total: {}",
            temp_struct.cube_count,
            temp_struct.limpet_count,
            temp_struct.cube_count + temp_struct.limpet_count
        );
    }

    if keyboard.pressed(KeyCode::ControlLeft) {
        let (entities, _): (Vec<Entity>, Vec<RenderComponent>) = render_query
            .iter()
            .filter(|&(_, r)| r.mesh == AssetId::CubeMesh)
            .unzip();

        if entities.len() > 0 {
            let rand_chosen = rng.random_range(0..entities.len());
            commands.entity(entities[rand_chosen]).despawn();
        }

        temp_struct.cube_count -= 1;
        println!(
            "cubes: {} limpets: {} total: {}",
            temp_struct.cube_count,
            temp_struct.limpet_count,
            temp_struct.cube_count + temp_struct.limpet_count
        );
    }

    if keyboard.pressed(KeyCode::ShiftLeft) {
        let spawn_pos: Vec3 = Vec3::new(
            rng.as_mut().random_range(-50.0..=50.0) as f32,
            rng.as_mut().random_range(-50.0..=50.0) as f32,
            rng.as_mut().random_range(-50.0..=50.0) as f32,
        );

        let random_texture: AssetId =
            AssetId::from_repr(rng.random_range(START_ENUM_T..=END_ENUM_T) as u16).unwrap();

        let random_size = rng.random_range(0.05..=2.0);

        commands.spawn((
            TransformComponent::new(
                spawn_pos,
                Vec3::new(random_size, random_size, random_size),
                Quat::from_euler(
                    glam::EulerRot::YXZ,
                    rng.random_range(-180.0..180.0),
                    rng.random_range(-180.0..180.0),
                    rng.random_range(-180.0..180.0),
                ),
            ),
            RenderComponent::new(
                AssetId::LimpetMesh,
                Material::new(
                    random_texture,
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
        ));

        temp_struct.limpet_count += 1;
        println!(
            "cubes: {} limpets: {} total: {}",
            temp_struct.cube_count,
            temp_struct.limpet_count,
            temp_struct.cube_count + temp_struct.limpet_count
        );
    }

    if keyboard.pressed(KeyCode::Tab) {
        let (entities, _): (Vec<Entity>, Vec<RenderComponent>) = render_query
            .iter()
            .filter(|&(_, r)| r.mesh == AssetId::LimpetMesh)
            .unzip();

        if entities.len() > 0 {
            let rand_chosen = rng.random_range(0..entities.len());
            commands.entity(entities[rand_chosen]).despawn();
        }

        temp_struct.limpet_count -= 1;
        println!(
            "cubes: {} limpets: {} total: {}",
            temp_struct.cube_count,
            temp_struct.limpet_count,
            temp_struct.cube_count + temp_struct.limpet_count
        );
    }
}

// fn keyboard_input_update(input_manager: Res<InputManager>) {
//     // Move input
//     if input_manager.move_axis_input != glam::Vec2::ZERO {
//         println!("Move Input: {}", input_manager.move_axis_input);
//     }

//     // Look input
//     if input_manager.look_axis_input != glam::Vec2::ZERO {
//         println!("Look Input: {}", input_manager.look_axis_input);
//     }

//     // Jump logs

//     if input_manager.jump_input.is_pressed {
//         println!("JUMP pressed");
//     }

//     if input_manager.jump_input.is_held {
//         println!(
//             "JUMP held for {{{}}} seconds",
//             input_manager.jump_input.hold_time
//         );
//     }

//     if input_manager.jump_input.is_just_released {
//         println!("JUMP released");
//         println!(
//             "JUMP was held for a total of {{{}}} seconds",
//             input_manager.jump_input.hold_time
//         );
//     }

//     // Shoot logs

//     if input_manager.shoot_input.is_pressed {
//         println!("SHOOT pressed");
//     }

//     if input_manager.shoot_input.is_held {
//         println!(
//             "SHOOT held for {{{}}} seconds",
//             input_manager.shoot_input.hold_time
//         );
//     }

//     if input_manager.shoot_input.is_just_released {
//         println!("SHOOT released");
//         println!(
//             "SHOOT was held for a total of {{{}}} seconds",
//             input_manager.shoot_input.hold_time
//         );
//     }
// }

fn main() {
    let mut bevy_app = bevy_app::App::new();
    bevy_app.add_plugins((
        EntropyPlugin::<WyRand>::default(),
        bevy_input::InputPlugin,
        bevy_gilrs::GilrsPlugin,
        bevy_time::TimePlugin,
    ));

    const DEFAULT_MOUSE_LOOK_SPEED: f32 = 100.0;
    const DEFAULT_GAMEPAD_LOOK_SPEED: f32 = 32.0;
    const DEFAULT_DEADZONES: f32 = 0.1;

    bevy_app.insert_resource(InputManager::new(
        glam::Vec2::new(DEFAULT_GAMEPAD_LOOK_SPEED, DEFAULT_GAMEPAD_LOOK_SPEED),
        DEFAULT_MOUSE_LOOK_SPEED,
        DEFAULT_DEADZONES,
        DEFAULT_DEADZONES,
    ));
    bevy_app.init_resource::<ButtonInput<KeyCode>>();
    bevy_app.init_resource::<ButtonInput<MouseButton>>();
    bevy_app.insert_resource(TempStruct {
        cube_count: 0,
        limpet_count: 0,
    });

    let event_loop = Arc::into_inner(
        VULKAN_RENDERER_SINGLETON
            .lock()
            .unwrap()
            .present_handle
            .window_handle
            .event_loop
            .take()
            .unwrap(),
    )
    .unwrap();

    bevy_app.add_systems(Startup, init_spawn).add_systems(
        Update,
        (
            //(keyboard_input_update,),
            temp_spawn_test,
            gameplay_update,
            (camera_move_system, camera_render_system).chain(),
        )
            .chain(),
    );

    let mut is_start = true;

    // Set a custom runner that drives the event loop and satisfies the return type
    bevy_app.set_runner(move |mut app| {
        event_loop
            .run(move |event, elwt| {
                match event {
                    Event::AboutToWait => {
                        let renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
                        renderer
                            .present_handle
                            .window_handle
                            .window
                            .request_redraw();
                    }

                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::KeyboardInput {
                            event: key_event, ..
                        } => {
                            if let PhysicalKey::Code(winit_key) = key_event.physical_key {
                                if let Some(mut keyboard_res) =
                                    app.world_mut().get_resource_mut::<ButtonInput<KeyCode>>()
                                {
                                    match key_event.state {
                                        ElementState::Pressed => keyboard_res.press(winit_key),
                                        ElementState::Released => keyboard_res.release(winit_key),
                                    }
                                }
                            }
                        }

                        WindowEvent::MouseInput { state, button, .. } => {
                            if let Some(mut mouse_res) = app
                                .world_mut()
                                .get_resource_mut::<ButtonInput<MouseButton>>()
                            {
                                match state {
                                    ElementState::Pressed => mouse_res.press(button),
                                    ElementState::Released => mouse_res.release(button),
                                }
                            }
                        }

                        WindowEvent::CursorMoved {
                            device_id: _,
                            position,
                        } => {
                            if let Some(mut input_manager) =
                                app.world_mut().get_resource_mut::<InputManager>()
                            {
                                input_manager.update_mouse_position(position);
                            }
                        }

                        WindowEvent::RedrawRequested if !elwt.exiting() => {
                            // 1. Start of frame updates from the renderer
                            if is_start {
                                is_start = false;
                            } else {
                                let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
                                renderer.start_of_frame_updates();
                            }

                            // 2. Register Inputs
                            {
                                let window = VULKAN_RENDERER_SINGLETON
                                    .lock()
                                    .unwrap()
                                    .present_handle
                                    .window_handle
                                    .window
                                    .clone();

                                let mut system_state: SystemState<(
                                    ResMut<InputManager>,
                                    Res<ButtonInput<KeyCode>>,
                                    Query<&Gamepad>,
                                    Res<ButtonInput<MouseButton>>,
                                    Res<Time>,
                                )> = SystemState::new(app.world_mut());

                                let (mut input_manager, keyboard, gamepads, mouse, time) =
                                    system_state.get_mut(app.world_mut()).unwrap();

                                input_manager.update(&keyboard, &gamepads, &mouse, &time, &window);
                            }

                            // 3. Update Time
                            if let Some(mut strategy) =
                                app.world_mut().get_resource_mut::<TimeUpdateStrategy>()
                            {
                                *strategy = TimeUpdateStrategy::ManualInstant(Instant::now());
                            }

                            // 4. Run systems
                            app.update();

                            // 5. Render
                            let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
                            renderer.render().unwrap();

                            // 6. Clear Inputs
                            if let Some(mut input_manager) =
                                app.world_mut().get_resource_mut::<InputManager>()
                            {
                                input_manager.clear_inputs();
                            }

                            if let Some(mut mouse_res) = app
                                .world_mut()
                                .get_resource_mut::<ButtonInput<MouseButton>>()
                            {
                                mouse_res.clear();
                            }

                            if let Some(mut keyboard_res) =
                                app.world_mut().get_resource_mut::<ButtonInput<KeyCode>>()
                            {
                                keyboard_res.clear();
                            }
                        }

                        WindowEvent::Resized(_size) => {
                            let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
                            renderer.present_handle.window_handle.is_resized = true;
                        }

                        WindowEvent::CloseRequested => {
                            let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
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

                    Event::DeviceEvent {
                        event: DeviceEvent::MouseMotion { delta: (dx, dy) },
                        ..
                    } => {
                        if let Some(mut input_manager) =
                            app.world_mut().get_resource_mut::<InputManager>()
                        {
                            input_manager.update_mouse_delta(dx, dy);
                        }
                    }

                    _ => {}
                }
            })
            .unwrap();

        bevy_app::AppExit::Success
    });

    bevy_app.run();
}
