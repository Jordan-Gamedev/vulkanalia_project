use glam::{Vec3, Vec4};

#[derive(Debug, Clone, Copy)]
pub enum InterpolationType {
    Linear,
    Step,
    CubicSpline,
}

#[derive(Debug, Clone, Copy)]
enum TargetType {
    ModelMatrix,
    Bone,
}

#[derive(Debug, Clone, Copy)]
enum PropertyType {
    Translation,
    Rotation,
    Scale,
    MorphTargetWeight,
}

#[derive(Debug, Clone)]
pub enum AnimationOutputData {
    Translations(Vec<Vec3>),
    Rotations(Vec<Vec4>),
    Scales(Vec<Vec3>),
    MorphTargetWeights(Vec<f32>),
}

#[derive(Debug, Clone)]
pub struct AnimationSampler {
    interpolation_type: InterpolationType,
    target_type: TargetType,
    target_property_type: PropertyType,
    timestamps: Vec<f32>,
    outputs: AnimationOutputData,
}

impl AnimationSampler {
    /// Gets the index of the input that is greater than or equal to the given time
    pub fn get_input_index_at(&self, time: f32) -> usize {
        self.inputs.partition_point(|&input| input <= time)
    }

    pub fn is_input_tweenable(&self, keyframe: f32) -> bool {
        keyframe != *self.inputs.last().unwrap() && keyframe != *self.inputs.first().unwrap()
    }

    pub fn update(
        &self,
        samplers: &Vec<AnimationSampler>,
        transform_component: &mut TransformComponent,
        current_time: f32,
    ) {
        // The sampler acting on this channel
        let sampler = &samplers[self.sampler_index as usize];

        // The target keyframe's index to interpolate towards
        let target_keyframe_index = sampler.get_input_index_at(current_time);

        if sampler.is_input_tweenable(sampler.inputs[target_keyframe_index]) {
            // The current keyframe
            let current_keyframe_index: usize = target_keyframe_index - 1;

            // Normalized time between the keyframes
            let t: f32 = (current_time - sampler.inputs[current_keyframe_index])
                / (sampler.inputs[target_keyframe_index] - sampler.inputs[current_keyframe_index]);

            match self.path_type {
                // Update position
                PathType::Translation => {
                    let start = sampler.outputs_vec_3[current_keyframe_index];
                    let end = sampler.outputs_vec_3[target_keyframe_index];
                    let mix = start.lerp_monotonic(end, t);

                    match self.target_type {
                        TargetType::ModelMatrix => {
                            transform_component.position += mix;
                            transform_component.update_model_matrix();
                        }
                        TargetType::Bone => {}
                    }
                }
                // Update rotation
                PathType::Rotation => {
                    let start = Quat::from_vec4(sampler.outputs_vec_4[current_keyframe_index]);
                    let end = Quat::from_vec4(sampler.outputs_vec_4[target_keyframe_index]);
                    let mix = start.slerp(end, t);

                    match self.target_type {
                        TargetType::ModelMatrix => {
                            transform_component.rotation *= mix;
                            transform_component.update_model_matrix();
                        }
                        TargetType::Bone => {}
                    }
                }
                // Update scale
                PathType::Scale => {
                    let start = sampler.outputs_vec_3[current_keyframe_index];
                    let end = sampler.outputs_vec_3[target_keyframe_index];
                    let mix = start.lerp_monotonic(end, t);

                    match self.target_type {
                        TargetType::ModelMatrix => {
                            transform_component.scale *= mix;
                            transform_component.update_model_matrix();
                        }
                        TargetType::Bone => {}
                    }
                }
            }
        }
    }
}
