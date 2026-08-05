#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuantizedModelMatrix {
    pub position: [f32; 3],
    pub scale: [f32; 3],
    pub rotation: [i16; 4],
}
