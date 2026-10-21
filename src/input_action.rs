use bevy_input::ButtonInput;
use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct KeyboardInputAction {
    pub bound_keys: Vec<KeyCode>,
    pub is_pressed: bool,
    pub is_just_released: bool,
    pub is_held: bool,
    pub hold_time: f32,
}

impl KeyboardInputAction {
    pub fn new(key_binds: Vec<KeyCode>) -> Self {
        Self {
            bound_keys: key_binds,
            is_pressed: false,
            is_just_released: false,
            is_held: false,
            hold_time: 0.0,
        }
    }

    pub fn update(&mut self, keyboard: &ButtonInput<KeyCode>, time: &bevy_time::Time) {
        if keyboard.any_just_pressed(self.bound_keys.clone()) {
            self.is_pressed = true;
        }

        if keyboard.any_just_released(self.bound_keys.clone()) {
            self.is_just_released = true;
        }

        if self.is_held {
            self.hold_time += time.delta_secs()
        } else {
            self.hold_time = 0.0;
        }

        self.is_held = keyboard.any_pressed(self.bound_keys.clone());
    }

    pub fn clear(&mut self) {
        self.is_pressed = false;
        self.is_just_released = false;
    }
}
