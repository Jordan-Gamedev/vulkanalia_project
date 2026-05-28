use vulkanalia::prelude::v1_0::*;
use vulkanalia_project::components::render::Render;
use vulkanalia_project::components::transform::Transform;
use vulkanalia_project::engine::App;
use vulkanalia_project::engine::texture_engine::{Material, SamplerContents};

fn main() {
    // Create the app
    let mut app = App::new().unwrap();

    for i in 0..2 {
        // Create entity with a renderer
        let entity = app.world.create_entity();
        let render_component = Render::new(
            "Limpet".to_string(),
            Material {
                albedo_name: "cuttlefish_albedo".to_string(),
                normal_ao_name: String::new(),
                metallic_roughness_emissive_name: String::new(),
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
        let transform_component = Transform::new();
        app.world.add_component(entity, render_component);
        app.world.add_component(entity, transform_component);
        let transform_component = *app.world.get_component::<Transform>(entity).unwrap();

        if i == 0 {
            transform_component.update_model_matrix(&mut app.world, glam::vec3(-2.0, 0.0, 0.0), glam::Quat::IDENTITY, glam::Vec3::ONE);    
        } else {
            transform_component.update_model_matrix(&mut app.world, glam::vec3(2.0, 0.0, 0.0), glam::Quat::IDENTITY, glam::Vec3::ONE);    
        }
    }

    // Run the app
    app.run();
}