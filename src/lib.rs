extern crate fftw;
extern crate libc;
extern crate libm;
extern crate num;
extern crate num_derive;
extern crate num_traits;
extern crate serde;
extern crate serde_json;

pub mod conf;
pub mod consts;
pub mod context;
mod effects;
mod errors;
mod factory;
pub mod ffi;
pub mod otters;
mod param;
pub mod traits;
mod utils;

#[cfg(test)]
mod test;

pub use otters::Otters;
pub use param::OttersParamModifierContext;