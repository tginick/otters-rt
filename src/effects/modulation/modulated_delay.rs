use crate::conf::{
    AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue, ParameterRange,
};
use crate::context::BoardContext;
use crate::traits::AudioEffect;
use crate::utils::{
    delay_buf::DelayBuffer,
    lfo::{bipolar_to_unipolar, LFOWaveForm, LowFrequencyOscillator},
    mathutils,
};

use crate::effects::basic_single_in_single_out;

use num_derive::FromPrimitive;
use std::cell::RefCell;
use std::fmt;

const PARAMS: &'static [AdvertisedParameter] = &[
    AdvertisedParameter {
        name: "mod_rate_hz",
        range: ParameterRange::F(0.02f32, 20.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.2f32),
    },
    AdvertisedParameter {
        name: "depth_pct",
        range: ParameterRange::F(0.0f32, 1.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.5f32),
    },
    AdvertisedParameter {
        name: "feedback_pct",
        range: ParameterRange::F(0.0f32, 1.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.5f32),
    },
];

const PARAM_MOD_RATE_HZ: usize = 0;
const PARAM_DEPTH_PCT: usize = 1;
const PARAM_FEEDBACK_PCT: usize = 2;

struct ModulatedDelayDerivedParameters {
    min_delay: f32,
    max_delay_depth: f32,
    dryness_db: f32,
    wetness_db: f32,
    actual_feedback_pct: f32,
    actual_effect_type: ModulatedDelayType,
}

#[allow(non_camel_case_types)]
#[derive(FromPrimitive, PartialEq)]
pub enum ModulatedDelayType {
    Flanger = 0,
    Chorus,
    Vibrato,
    WhiteChorus,

    __NUM_MODULATED_DELAY_TYPES,
}

pub struct ModulatedDelay {
    params: Vec<BoardEffectConfigParameterValue>,
    derived_params: ModulatedDelayDerivedParameters,

    delay_buf: RefCell<DelayBuffer>,
    lfo: RefCell<LowFrequencyOscillator>,
}

impl Default for ModulatedDelayType {
    fn default() -> Self {
        ModulatedDelayType::Flanger
    }
}

impl fmt::Display for ModulatedDelayType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModulatedDelayType::Flanger => write!(f, "Flanger"),
            ModulatedDelayType::Chorus => write!(f, "Chorus"),
            ModulatedDelayType::Vibrato => write!(f, "Vibrato"),
            ModulatedDelayType::WhiteChorus => write!(f, "WhiteChorus"),

            ModulatedDelayType::__NUM_MODULATED_DELAY_TYPES => {
                write!(f, "!InvalidModulatedDelayType")
            }
        }
    }
}

impl ModulatedDelay {
    pub fn new_flanger(ac: AudioConfig) -> ModulatedDelay {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        let derived_params = flanger_params(&params);

        ModulatedDelay {
            delay_buf: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
            lfo: RefCell::new(LowFrequencyOscillator::new(
                LFOWaveForm::Triangle,
                PARAMS[PARAM_MOD_RATE_HZ].default_value.as_flt(),
                ac.sample_rate,
            )),
            params,
            derived_params,
        }
    }

    pub fn new_chorus(ac: AudioConfig) -> ModulatedDelay {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        let derived_params = chorus_params(&params);

        ModulatedDelay {
            delay_buf: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
            lfo: RefCell::new(LowFrequencyOscillator::new(
                LFOWaveForm::Triangle,
                PARAMS[PARAM_MOD_RATE_HZ].default_value.as_flt(),
                ac.sample_rate,
            )),
            params,
            derived_params,
        }
    }

    pub fn new_vibrato(ac: AudioConfig) -> ModulatedDelay {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        let derived_params = vibrato_params(&params);

        ModulatedDelay {
            delay_buf: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
            lfo: RefCell::new(LowFrequencyOscillator::new(
                LFOWaveForm::Sine, // vibrato uses a sine LFO instead of a triangle one
                PARAMS[PARAM_MOD_RATE_HZ].default_value.as_flt(),
                ac.sample_rate,
            )),
            params,
            derived_params,
        }
    }

    pub fn new_white_chorus(ac: AudioConfig) -> ModulatedDelay {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        let derived_params = white_chorus_params(&params);
        ModulatedDelay {
            delay_buf: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
            lfo: RefCell::new(LowFrequencyOscillator::new(
                LFOWaveForm::Triangle,
                PARAMS[PARAM_MOD_RATE_HZ].default_value.as_flt(),
                ac.sample_rate,
            )),
            params,
            derived_params,
        }
    }

    pub fn modulated_delay_info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for ModulatedDelay {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        ModulatedDelay::modulated_delay_info()
    }

    fn set_audio_parameters(&mut self, new_config: &AudioConfig) {
        self.lfo
            .borrow_mut()
            .change_sample_rate(new_config.sample_rate);

        self.delay_buf
            .borrow_mut()
            .change_sample_rate(new_config.sample_rate);
    }

    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;
    }

    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        let mut lfo = self.lfo.borrow_mut();
        let (read_buf, mut write_buf) = maybe_bufs.unwrap();
        let delay_min_ms = self.derived_params.min_delay;
        let delay_max_ms = self.derived_params.min_delay + self.derived_params.max_delay_depth;
        let depth = self.params[PARAM_DEPTH_PCT].as_flt();
        let feedback = self.derived_params.actual_feedback_pct;
        let dryness = mathutils::db_to_linear(self.derived_params.dryness_db);
        let wetness = mathutils::db_to_linear(self.derived_params.wetness_db);

        let mut delay_ref = self.delay_buf.borrow_mut();

        for i in 0..num_samples {
            let real_delay_ms = if self.derived_params.actual_effect_type
                == ModulatedDelayType::Flanger
            {
                mathutils::lerp(
                    delay_min_ms,
                    delay_max_ms,
                    bipolar_to_unipolar(depth * lfo.current_sample()),
                )
            } else {
                mathutils::bipolar_lerp(delay_min_ms, delay_max_ms, depth * lfo.current_sample())
            };

            // advance lfo
            lfo.oscillate();

            // adjust delay
            delay_ref.set_delay_time_ms(real_delay_ms, true);

            // do actual delay
            let xn = read_buf.buf_read(i);
            let yn = delay_ref.read_delayed_sample();
            let dn = xn + feedback * yn;

            delay_ref.write_sample(dn);

            let on = dryness * xn + wetness * yn;
            write_buf.buf_write(i, on);
        }
    }
}

fn flanger_params(
    effect_params: &Vec<BoardEffectConfigParameterValue>,
) -> ModulatedDelayDerivedParameters {
    ModulatedDelayDerivedParameters {
        min_delay: 0.1f32,
        max_delay_depth: 7.0f32,
        dryness_db: -3.0f32,
        wetness_db: -3.0f32,
        actual_feedback_pct: effect_params[PARAM_FEEDBACK_PCT].as_flt(),
        actual_effect_type: ModulatedDelayType::Flanger,
    }
}

fn vibrato_params(
    _effect_params: &Vec<BoardEffectConfigParameterValue>,
) -> ModulatedDelayDerivedParameters {
    ModulatedDelayDerivedParameters {
        min_delay: 0.0f32,
        max_delay_depth: 7.0f32,
        dryness_db: -96.0f32,
        wetness_db: 0.0f32,
        actual_feedback_pct: 0.0f32,
        actual_effect_type: ModulatedDelayType::Vibrato,
    }
}

fn chorus_params(
    _effect_params: &Vec<BoardEffectConfigParameterValue>,
) -> ModulatedDelayDerivedParameters {
    ModulatedDelayDerivedParameters {
        min_delay: 10.0f32,
        max_delay_depth: 30.0f32,
        dryness_db: 0.0f32,
        wetness_db: -3.0f32,
        actual_feedback_pct: 0.0f32,
        actual_effect_type: ModulatedDelayType::Chorus,
    }
}

fn white_chorus_params(
    _effect_params: &Vec<BoardEffectConfigParameterValue>,
) -> ModulatedDelayDerivedParameters {
    ModulatedDelayDerivedParameters {
        min_delay: 7.0f32,
        max_delay_depth: 30.0f32,
        dryness_db: 0.0f32,
        wetness_db: -3.0f32,
        actual_feedback_pct: -0.7f32,
        actual_effect_type: ModulatedDelayType::WhiteChorus,
    }
}
