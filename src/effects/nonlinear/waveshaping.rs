use crate::conf::{AdvertisedParameter, BoardEffectConfigParameterValue, ParameterRange};
use crate::effects::basic_single_in_single_out;
use crate::{
    traits::AudioEffect,
    utils::mathutils::{vatan, vtanh},
};
use num_derive::FromPrimitive;
use std::fmt;

const PARAMS: &'static [AdvertisedParameter] = &[
    AdvertisedParameter {
        name: "waveshaper_function",
        range: ParameterRange::N(0, WaveShaperFunction::__NUM_FUNCTIONS as i32),
        default_value: BoardEffectConfigParameterValue::N(0),
    },
    AdvertisedParameter {
        name: "gain",
        range: ParameterRange::F(1.0f32, 64.0f32),
        default_value: BoardEffectConfigParameterValue::F(4.0f32),
    },
];

const PARAM_WAVESHAPER_FUNCTION: usize = 0;
const PARAM_GAIN: usize = 1;

#[derive(Clone, Copy, FromPrimitive)]
#[allow(non_camel_case_types)]
pub enum WaveShaperFunction {
    Identity = 0,
    Arraya,
    Sigmoid,
    HyperbolicTangent,
    InverseTangent,
    FuzzExponential,
    FuzzExponential2,
    ArctangentSquareRoot,
    SquareSign,
    HardClip,
    HalfRectifier,
    FullRectifier,

    __NUM_FUNCTIONS,
}

impl Default for WaveShaperFunction {
    fn default() -> Self {
        WaveShaperFunction::Identity
    }
}

// : splits name and comma-separated list of attributes
// NG attribute - ignores gain parameter
// X attribute - exotic waveshaper function (weird results)
impl fmt::Display for WaveShaperFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaveShaperFunction::Identity => write!(f, "Identity"),
            WaveShaperFunction::Arraya => write!(f, "Arraya:NG"),
            WaveShaperFunction::Sigmoid => write!(f, "Sigmoid"),
            WaveShaperFunction::HyperbolicTangent => write!(f, "HyperbolicTangent"),
            WaveShaperFunction::InverseTangent => write!(f, "InverseTangent"),
            WaveShaperFunction::FuzzExponential => write!(f, "FuzzExponential"),
            WaveShaperFunction::FuzzExponential2 => write!(f, "FuzzExponential2:NG,X"),
            WaveShaperFunction::ArctangentSquareRoot => write!(f, "ATSR:NG,X"),
            WaveShaperFunction::SquareSign => write!(f, "SquareSign:NG,X"),
            WaveShaperFunction::HardClip => write!(f, "HardClip:X"),
            WaveShaperFunction::HalfRectifier => write!(f, "Half Wave Rectifier:NG,X"),
            WaveShaperFunction::FullRectifier => write!(f, "Full Wave Rectifier:NG,X"),

            WaveShaperFunction::__NUM_FUNCTIONS => write!(f, "!InvalidWaveShaperFunction"),
        }
    }
}

pub struct WaveShaper {
    params: Vec<BoardEffectConfigParameterValue>,
    real_waveshaper_function: WaveShaperFunction,
}

impl WaveShaper {
    pub fn new() -> WaveShaper {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        WaveShaper {
            params,
            real_waveshaper_function: WaveShaperFunction::Identity,
        }
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for WaveShaper {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        WaveShaper::info()
    }
    fn set_audio_parameters(&mut self, _new_config: &crate::conf::AudioConfig) {}
    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;

        if param_idx == PARAM_WAVESHAPER_FUNCTION {
            self.real_waveshaper_function = param_value.as_enum();
        }
    }
    fn execute(
        &self,
        context: &crate::context::BoardContext,
        connection_idx: usize,
        num_samples: usize,
    ) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        // actual processing
        // TODO: Low-hanging fruit for vectorization
        let (read_buf, mut write_buf) = maybe_bufs.unwrap();
        for i in 0..num_samples {
            let s = read_buf.buf_read(i);
            write_buf.buf_write(
                i,
                execute_waveshaper_function(
                    self.real_waveshaper_function,
                    self.params[PARAM_GAIN].as_flt(),
                    s,
                ),
            );
        }
    }
}

fn execute_waveshaper_function(function: WaveShaperFunction, gain: f32, sample: f32) -> f32 {
    let v = match function {
        WaveShaperFunction::Identity => sample,
        WaveShaperFunction::Arraya => ws_arraya(sample),
        WaveShaperFunction::Sigmoid => ws_sigmoid(gain, sample),
        WaveShaperFunction::HyperbolicTangent => ws_tanh(gain, sample),
        WaveShaperFunction::InverseTangent => ws_atan(gain, sample),
        WaveShaperFunction::FuzzExponential => ws_fuzz_exp(gain, sample),
        WaveShaperFunction::FuzzExponential2 => x_ws_fuzz_exp_2(sample),

        // TODO: playing around with these parameters may be useful
        WaveShaperFunction::ArctangentSquareRoot => {
            x_ws_atsr(sample, 2.5f32, 0.9f32, 2.5f32, 0.9f32)
        }

        WaveShaperFunction::SquareSign => x_ws_sqs(sample),

        // TODO clip location should be configurable
        // maybe just extract to a separate unit?
        WaveShaperFunction::HardClip => x_ws_hclip(gain, sample, 0.5f32),

        WaveShaperFunction::HalfRectifier => x_ws_half_rec(sample),
        WaveShaperFunction::FullRectifier => x_ws_full_rec(sample),

        _ => 0f32,
    };

    v.max(-1.0f32).min(1.0f32)
}

fn ws_arraya(sample: f32) -> f32 {
    (3.0f32 * sample / 2.0f32) * (1.0f32 - sample * sample / 3.0f32)
}

fn ws_sigmoid(gain: f32, sample: f32) -> f32 {
    2.0f32 * (1.0f32 / (1.0f32 + (-gain * sample).exp())) - 1.0f32
}

fn ws_tanh(gain: f32, sample: f32) -> f32 {
    vtanh(gain * sample) / vtanh(gain)
}

fn ws_atan(gain: f32, sample: f32) -> f32 {
    vatan(gain * sample) / vatan(gain)
}

fn ws_fuzz_exp(gain: f32, sample: f32) -> f32 {
    sample.signum() * (1.0f32 - (-(gain * sample).abs()).exp()) / (1.0f32 - (-gain).exp())
}

fn x_ws_fuzz_exp_2(sample: f32) -> f32 {
    (-sample).signum() * (1.0f32 - sample.abs()) / (std::f32::consts::E - 1.0f32)
}

fn x_ws_atsr(sample: f32, alpha: f32, beta: f32, psi: f32, zeta: f32) -> f32 {
    alpha * vatan(beta * sample) + psi * (1.0f32 - (zeta * zeta * sample * sample)).sqrt() - psi
}

fn x_ws_sqs(sample: f32) -> f32 {
    sample * sample * sample.signum()
}

fn x_ws_hclip(gain: f32, sample: f32, clip_at: f32) -> f32 {
    // TODO: i don't like this conditional
    if gain * sample.abs() > clip_at {
        clip_at * sample.signum()
    } else {
        gain * sample
    }
}

fn x_ws_half_rec(sample: f32) -> f32 {
    0.5f32 * (sample + sample.abs())
}

fn x_ws_full_rec(sample: f32) -> f32 {
    sample.abs()
}
