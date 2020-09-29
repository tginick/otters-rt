pub fn vmodf(v: f32) -> (i32, f32) {
    (v.trunc() as i32, v.fract())
}

pub fn vsinf(v: f32) -> f32 {
    v.sin()
}

pub fn vcosf(v: f32) -> f32 {
    v.cos()
}

pub fn vtanf(v: f32) -> f32 {
    v.tan()
}

pub fn vtanh(v: f32) -> f32 {
    v.tanh()
}

pub fn vatan(v: f32) -> f32 {
    v.atan()
}

pub fn vexpf(v: f32) -> f32 {
    v.exp()
}

pub fn vsqrtf(v: f32) -> f32 {
    v.powf(0.5f32)
}