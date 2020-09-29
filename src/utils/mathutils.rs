#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ops::BitAnd;

#[cfg(target_arch = "arm")]
use std::os::raw;

#[cfg(target_arch = "arm")]
include!("arch/arm.rs");

#[cfg(target_arch = "aarch64")]
include!("arch/generic.rs");

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
include!("arch/generic.rs");

pub fn nextafter(a: f32, b: f32) -> f32 {
    libm::nextafterf(a, b)
}

// t is [0, 1]
pub fn lerp(x: f32, y: f32, t: f32) -> f32 {
    x + t * (y - x)
}

// t is [-1, 1]
pub fn bipolar_lerp(x: f32, y: f32, t: f32) -> f32 {
    let half = (y - x) / 2.0f32;
    let mid = x + half;
    t * half + mid
}

pub fn db_to_linear(db: f32) -> f32 {
    10.0f32.powf(db / 20.0f32)
}

// wow this is both really ugly and kinda nice at the same time somehow
pub fn is_power_of_2<S, T>(v: T) -> bool
where
    S: num::Integer + PartialEq + Copy,
    T: num::Integer + BitAnd<Output = S> + PartialEq<S> + Copy,
{
    if v == T::zero() {
        return false; // 0 is not a power of 2
    }

    return v & (v - T::one()) == S::zero();
}
