include!(concat!(env!("OUT_DIR"), "/math_neon.rs"));

pub fn vsinf(v: f32) -> f32 {
    unsafe { arm::sinf_neon_hfp(v) }
}

pub fn vcosf(v: f32) -> f32 {
    unsafe { arm::cosf_neon_hfp(v) }
}

pub fn vmodf(v: f32) -> (i32, f32) {
    let mut ipart: raw::c_int = 0;
    let fpart = unsafe { arm::modf_neon_hfp(v, &mut ipart as *mut raw::c_int) };

    (ipart as i32, fpart)
}

pub fn vtanf(v: f32) -> f32 {
    unsafe { arm::tanf_neon_hfp(v) }
}

pub fn vtanh(v: f32) -> f32 {
    unsafe { arm::tanhf_neon_hfp(v) }
}

pub fn vatan(v: f32) -> f32 {
    unsafe { arm::atanf_neon_hfp(v) }
}

pub fn vexpf(v: f32) -> f32 {
    unsafe { arm::expf_neon_hfp(v) }
}

pub fn vsqrtf(v: f32) -> f32 {
    v.powf(0.5f32)
}
