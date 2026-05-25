use vulkanalia::prelude::v1_0::*;
use vulkanalia_project::components::render::Render;
use vulkanalia_project::engine::App;
use vulkanalia_project::engine::texture_engine::{Material, SamplerContents};

fn main() {
    // Create the app
    let mut app = App::new().unwrap();

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
    app.world.add_component(entity, render_component);

    // Run the app
    app.run();
}