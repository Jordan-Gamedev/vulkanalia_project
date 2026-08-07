use crate::components::TransformComponent;
use crate::engine::Material;
use crate::engine::QuantizedModelMatrix;
use crate::engine::VulkanRenderer;
use crate::resources::AssetId;
use anyhow::Result;
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::DeferredWorld;
use glam::Quat;
use glam::Vec3;

#[derive(Component)]
#[require(TransformComponent::default())]
#[component(on_add = Self::on_add, on_remove = Self::on_remove)]
pub struct RenderComponent {
    pub model_vertices: AssetId, // An asset reference to the chosen model's vertices
    pub model_indices: AssetId,  // An asset reference to the chosen model's indices
    pub material: Material,      // The material that this entity uses
    pub model_matrix_info: u32,  // The model matrix
    pub is_receiving_shadows: bool, // Whether this entity should receive shadows from other shadow casters
    pub is_casting_shadows: bool,   // Whether this entity is a shadow caster
}

impl RenderComponent {
    pub fn new(
        model_vertices: AssetId,
        model_indices: AssetId,
        material: Material,
        receives_shadows: bool,
        casts_shadows: bool,
    ) -> Self {
        Self {
            model_vertices,
            model_indices,
            material,
            model_matrix_info: u32::MAX,
            is_receiving_shadows: receives_shadows,
            is_casting_shadows: casts_shadows,
        }
    }

    fn on_add(mut world: DeferredWorld, hook_context: HookContext) {
        // Get an unsafe world cell view
        let cell = world.as_unsafe_world_cell();

        // Fetch the resource directly from cell
        let mut vulkan_renderer = unsafe { cell.get_resource_mut::<VulkanRenderer>().unwrap() };

        // Update transform component
        let mut transform_component = unsafe {
            cell.get_entity(hook_context.entity)
                .unwrap()
                .get_mut::<TransformComponent>()
                .unwrap()
        };
        transform_component.scale = Vec3::new(0.01, 0.01, 0.01);

        // Create model matrix
        let model_matrix_index = vulkan_renderer
            .create_model_matrix(
                transform_component.to_quantized_matrix(),
                transform_component.is_static,
            )
            .unwrap();

        // Fetch the render component directly from cell
        let mut render = unsafe {
            cell.get_entity(hook_context.entity)
                .unwrap()
                .get_mut::<RenderComponent>()
                .unwrap()
        };
        render.set_model_matrix_index(model_matrix_index);
        render.set_is_static(false);
        transform_component.model_matrix_index = model_matrix_index;

        // Create instance
        vulkan_renderer
            .add_instance(
                render.model_vertices,
                render.model_indices,
                render.material.albedo,
                render.material.sampler_contents,
                render.model_matrix_info,
            )
            .unwrap();
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

        // Fetch the resource directly from cell
        let mut vulkan_renderer = unsafe { cell.get_resource_mut::<VulkanRenderer>().unwrap() };

        // Remove instance and model matrix
        unsafe {
            for i in 0..vulkan_renderer.command_handle.instance_buffer.capacity {
                let instance = vulkan_renderer.command_handle.instance_buffer.mapped.add(i);
                if instance.read().model_matrix_info == render.model_matrix_info {
                    vulkan_renderer
                        .remove_instance(
                            render.model_vertices,
                            render.model_indices,
                            render.material.albedo,
                            render.material.sampler_contents,
                            instance,
                        )
                        .unwrap();
                }
            }
        }
        vulkan_renderer
            .remove_model_matrix(render.get_model_matrix_index(), render.is_static())
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

    pub fn get_model_matrix_index(&self) -> u32 {
        self.model_matrix_info & 0x7FFFFFFF
    }

    pub fn get_quantized_model_matrix(
        &self,
        vulkan_renderer: &VulkanRenderer,
    ) -> Result<QuantizedModelMatrix> {
        let model_matrix =
            vulkan_renderer.get_model_matrix(self.get_model_matrix_index(), self.is_static());
        Ok(model_matrix)
    }

    pub fn is_static(&self) -> bool {
        self.model_matrix_info & 0x80000000 > 0
    }

    pub fn set_model_matrix(
        &self,
        vulkan_renderer: &mut VulkanRenderer,
        position: Vec3,
        rotation: Quat,
        scale: Vec3,
    ) {
        vulkan_renderer
            .set_model_matrix(
                self.get_model_matrix_index(),
                position,
                rotation,
                scale,
                self.is_static(),
            )
            .unwrap();
    }

    /// Mark this render transform as static (only works before adding component)
    pub fn set_is_static(&mut self, is_static: bool) {
        self.model_matrix_info &= 0x7FFFFFFF;
        self.model_matrix_info |= (is_static as u32) << 31;
    }

    fn set_model_matrix_index(&mut self, val: u32) {
        self.model_matrix_info = (self.model_matrix_info & 0x80000000) | val;
    }
}
