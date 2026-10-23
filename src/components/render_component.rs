use crate::components::TransformComponent;
use crate::engine::Material;
use crate::engine::vulkan_renderer::VULKAN_RENDERER_SINGLETON;
use crate::resources::AssetId;
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::DeferredWorld;

#[derive(Component, Clone, Copy)]
#[require(TransformComponent::default())]
#[component(on_add = Self::on_add, on_remove = Self::on_remove)]
pub struct RenderComponent {
    pub mesh: AssetId,                 // An asset reference to the renderer's mesh
    pub material: Material,            // The material that this entity uses
    pub instance_transform_index: u32, // The instance and model matrix index
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool,   // Whether this entity is a shadow caster
}

impl RenderComponent {
    pub fn new(
        mesh: AssetId,
        material: Material,
        receives_shadows: bool,
        casts_shadows: bool,
    ) -> Self {
        Self {
            mesh,
            material,
            instance_transform_index: u32::MAX,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
        }
    }

    fn on_add(mut world: DeferredWorld, hook_context: HookContext) {
        // Get an unsafe world cell view
        let cell = world.as_unsafe_world_cell();

        // Get the renderer
        let vulkan_renderer = &mut VULKAN_RENDERER_SINGLETON.lock().unwrap();

        // Update transform component
        let mut transform_component = unsafe {
            cell.get_entity(hook_context.entity)
                .unwrap()
                .get_mut::<TransformComponent>()
                .unwrap()
        };

        // Fetch the render component directly from cell
        let mut render = unsafe {
            cell.get_entity(hook_context.entity)
                .unwrap()
                .get_mut::<RenderComponent>()
                .unwrap()
        };

        let mut render_component_query =
            unsafe { cell.world_mut().query::<&mut RenderComponent>() };
        let query_iter: bevy_ecs::query::QueryIter<'_, '_, &mut RenderComponent, ()> =
            unsafe { render_component_query.iter_mut(cell.world_mut()) };

        // Create instance
        let (_, instance_model_matrix_index) = vulkan_renderer
            .add_instance(
                query_iter,
                render.mesh,
                render.material.albedo,
                render.material.sampler_contents,
                transform_component.to_quantized_matrix(),
            )
            .unwrap();

        render.instance_transform_index = instance_model_matrix_index as u32;
        transform_component.model_matrix_index = instance_model_matrix_index as u32;
    }

    fn on_remove(mut world: DeferredWorld, hook_context: HookContext) {
        // Get an unsafe world cell view
        let cell = world.as_unsafe_world_cell();

        // Fetch the component directly from cell
        let render = unsafe {
            cell.get_entity(hook_context.entity)
                .unwrap()
                .get::<RenderComponent>()
                .unwrap()
        };

        // Get the renderer
        let vulkan_renderer = &mut VULKAN_RENDERER_SINGLETON.lock().unwrap();

        let mut render_component_query =
            unsafe { cell.world_mut().query::<&mut RenderComponent>() };
        let query_iter: bevy_ecs::query::QueryIter<'_, '_, &mut RenderComponent, ()> =
            unsafe { render_component_query.iter_mut(cell.world_mut()) };

        // Remove instance
        vulkan_renderer
            .remove_instance(
                query_iter,
                render.mesh,
                render.material.albedo,
                render.material.sampler_contents,
                render.instance_transform_index,
            )
            .unwrap();

        // Transform no longer has a model matrix index or a render
        let mut transform = unsafe {
            cell.get_entity(hook_context.entity)
                .unwrap()
                .get_mut::<TransformComponent>()
                .unwrap()
        };
        transform.model_matrix_index = u32::MAX;
    }
}
