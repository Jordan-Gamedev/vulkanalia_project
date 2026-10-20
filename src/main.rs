#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unused_imports)]
use bevy_app::{Startup, Update};
use bevy_ecs::prelude::*;
use std::sync::Arc;
use vulkanalia::prelude::v1_0::*;
use vulkanalia_project::components::RenderComponent;
use vulkanalia_project::engine::Material;
use vulkanalia_project::engine::SamplerContents;
use vulkanalia_project::engine::VulkanRenderer;
use vulkanalia_project::engine::vulkan_renderer::VULKAN_RENDERER_SINGLETON;
use vulkanalia_project::prefabs::*;
use vulkanalia_project::resources::*;
use winit::event::{Event, WindowEvent};

// Runs exactly once at startup
fn init_spawn(mut commands: Commands) {
    commands.spawn(CubePrefab::component_bundle());
}

fn gameplay_update(mut _commands: Commands) {
    //println!("UPDATE");
}

fn main() {
    let mut bevy_app = bevy_app::App::new();

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

    bevy_app
        .add_systems(Startup, init_spawn)
        .add_systems(Update, gameplay_update);

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
                        WindowEvent::RedrawRequested if !elwt.exiting() => {
                            // 1. Start of frame updates from the renderer
                            {
                                if is_start {
                                    is_start = false;
                                } else {
                                    let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
                                    renderer.start_of_frame_updates();
                                }
                            }

                            // 2. Run systems
                            app.update();

                            // 3. Render
                            let mut renderer = VULKAN_RENDERER_SINGLETON.lock().unwrap();
                            renderer.render().unwrap();
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
                    _ => {}
                }
            })
            .unwrap();

        bevy_app::AppExit::Success
    });

    bevy_app.run();
}
