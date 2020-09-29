use num::FromPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct AudioConfig {
    pub sample_rate: f32,
    pub max_block_size: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum BoardEffectConfigParameterValue {
    N(i32),
    F(f32),
}

impl BoardEffectConfigParameterValue {
    pub fn as_int(&self) -> i32 {
        match *self {
            BoardEffectConfigParameterValue::N(x) => x,
            BoardEffectConfigParameterValue::F(x) => x.round() as i32,
        }
    }

    pub fn as_flt(&self) -> f32 {
        match *self {
            BoardEffectConfigParameterValue::N(x) => x as f32,
            BoardEffectConfigParameterValue::F(x) => x,
        }
    }

    pub fn as_enum<T>(&self) -> T
    where
        T: FromPrimitive + Default,
    {
        let y = match *self {
            BoardEffectConfigParameterValue::N(x) => T::from_i32(x),
            BoardEffectConfigParameterValue::F(x) => T::from_i32(x.round() as i32),
        };

        y.unwrap_or(T::default())
    }
}

#[derive(Copy, Clone, Serialize)]
pub enum ParameterRange {
    N(i32, i32),
    F(f32, f32),
}

#[derive(Copy, Clone, Serialize)]
pub struct AdvertisedParameter {
    pub name: &'static str,
    pub range: ParameterRange,
    pub default_value: BoardEffectConfigParameterValue,
}

#[derive(Serialize, Deserialize)]
pub struct BoardEffectConfigParameter {
    pub name: String,
    pub value: BoardEffectConfigParameterValue,
}

#[derive(Serialize, Deserialize)]
pub struct BoardEffectDeclaration {
    pub effect_name: String,
    pub bind_name: String,
    pub config: Vec<BoardEffectConfigParameter>,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BoardConnectionDeclaration {
    pub effect: String,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BoardConfig {
    pub buffers: Vec<String>,
    pub effects: Vec<BoardEffectDeclaration>,
    pub connections: Vec<BoardConnectionDeclaration>,
}
