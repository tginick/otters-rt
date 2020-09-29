use crate::conf::{
    AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue, ParameterRange,
};
use crate::consts::MAX_DELAY_MS;
use crate::context::BoardContext;
use crate::traits::AudioEffect;

use crate::effects::basic_single_in_single_out;
use crate::utils::delay_buf::DelayBuffer;
use crate::utils::envelope::EnvelopeDetector;
use crate::utils::mathutils;

use num_derive::ToPrimitive;
use num_traits::ToPrimitive;

use std::cell::RefCell;

const PARAMS: &'static [AdvertisedParameter] = &[
    AdvertisedParameter {
        name: "threshold_db",
        range: ParameterRange::F(-40.0f32, 0.0f32),
        default_value: BoardEffectConfigParameterValue::F(-10.0f32),
    },
    AdvertisedParameter {
        name: "knee_width_db",
        range: ParameterRange::F(0.0f32, 20.0f32),
        default_value: BoardEffectConfigParameterValue::F(5.0f32),
    },
    AdvertisedParameter {
        name: "ratio",
        range: ParameterRange::F(1.0f32, 20.0f32),
        default_value: BoardEffectConfigParameterValue::F(1.0f32),
    },
    AdvertisedParameter {
        name: "attack_time_ms",
        range: ParameterRange::F(1.0f32, 100.0f32),
        default_value: BoardEffectConfigParameterValue::F(5.0f32),
    },
    AdvertisedParameter {
        name: "release_time_ms",
        range: ParameterRange::F(1.0f32, 5000.0f32),
        default_value: BoardEffectConfigParameterValue::F(500.0f32),
    },
    AdvertisedParameter {
        name: "output_gain_db",
        range: ParameterRange::F(-20.0f32, 20.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.0f32),
    },
    AdvertisedParameter {
        name: "soft_knee?",
        range: ParameterRange::N(0, 1),
        default_value: BoardEffectConfigParameterValue::N(1),
    },
    AdvertisedParameter {
        name: "delay_ms",
        range: ParameterRange::F(0.0f32, MAX_DELAY_MS),
        default_value: BoardEffectConfigParameterValue::F(0.0f32),
    },
];

const GAIN_FNS: &'static [fn(f32, &Vec<BoardEffectConfigParameterValue>) -> f32] = &[
    calculate_compressor_gain_hard_knee,
    calculate_limiter_gain_hard_knee,
    calculate_expander_gain_hard_knee,
    calculate_gate_gain_hard_knee,
    calculate_compressor_gain_soft_knee,
    calculate_limiter_gain_soft_knee,
    calculate_expander_gain_soft_knee,
    calculate_gate_gain_soft_knee,
];

const PARAM_THRESHOLD_DB: usize = 0;
const PARAM_KNEE_WIDTH_DB: usize = 1;
const PARAM_RATIO: usize = 2;
const PARAM_ATTACK_TIME_MS: usize = 3;
const PARAM_RELEASE_TIME_MS: usize = 4;
const PARAM_OUTPUT_GAIN_DB: usize = 5;
const PARAM_SOFT_KNEE: usize = 6;
const PARAM_DELAY_MS: usize = 7;

#[derive(ToPrimitive)]
pub enum DynamicsProcessorType {
    Compressor = 0,
    Limiter,
    Expander,
    Gate,
}

pub struct Dynamics {
    params: Vec<BoardEffectConfigParameterValue>,
    envelope_detector: EnvelopeDetector,
    real_output_gain: f32,
    processor_type: DynamicsProcessorType,
    delay: RefCell<DelayBuffer>,
}

impl Dynamics {
    pub fn new_compressor(ac: AudioConfig) -> Dynamics {
        let params = Dynamics::init_params();
        let mut ed = EnvelopeDetector::new(ac.sample_rate);
        ed.set_attack_time_ms(params[PARAM_ATTACK_TIME_MS].as_flt());
        ed.set_release_time_ms(params[PARAM_RELEASE_TIME_MS].as_flt());

        let output_gain_db = params[PARAM_OUTPUT_GAIN_DB].as_flt();

        Dynamics {
            params,
            envelope_detector: ed,
            real_output_gain: mathutils::db_to_linear(output_gain_db),
            processor_type: DynamicsProcessorType::Compressor,
            delay: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
        }
    }

    pub fn new_expander(ac: AudioConfig) -> Dynamics {
        let params = Dynamics::init_params();
        let mut ed = EnvelopeDetector::new(ac.sample_rate);
        ed.set_attack_time_ms(params[PARAM_ATTACK_TIME_MS].as_flt());
        ed.set_release_time_ms(params[PARAM_RELEASE_TIME_MS].as_flt());

        let output_gain_db = params[PARAM_OUTPUT_GAIN_DB].as_flt();

        Dynamics {
            params,
            envelope_detector: ed,
            real_output_gain: mathutils::db_to_linear(output_gain_db),
            processor_type: DynamicsProcessorType::Expander,
            delay: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
        }
    }

    pub fn new_limiter(ac: AudioConfig) -> Dynamics {
        let params = Dynamics::init_params();
        let mut ed = EnvelopeDetector::new(ac.sample_rate);
        ed.set_attack_time_ms(params[PARAM_ATTACK_TIME_MS].as_flt());
        ed.set_release_time_ms(params[PARAM_RELEASE_TIME_MS].as_flt());

        let output_gain_db = params[PARAM_OUTPUT_GAIN_DB].as_flt();

        Dynamics {
            params,
            envelope_detector: ed,
            real_output_gain: mathutils::db_to_linear(output_gain_db),
            processor_type: DynamicsProcessorType::Limiter,
            delay: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
        }
    }

    pub fn new_gate(ac: AudioConfig) -> Dynamics {
        let params = Dynamics::init_params();
        let mut ed = EnvelopeDetector::new(ac.sample_rate);
        ed.set_attack_time_ms(params[PARAM_ATTACK_TIME_MS].as_flt());
        ed.set_release_time_ms(params[PARAM_RELEASE_TIME_MS].as_flt());

        let output_gain_db = params[PARAM_OUTPUT_GAIN_DB].as_flt();

        Dynamics {
            params,
            envelope_detector: ed,
            real_output_gain: mathutils::db_to_linear(output_gain_db),
            processor_type: DynamicsProcessorType::Gate,
            delay: RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate)),
        }
    }

    fn init_params() -> Vec<BoardEffectConfigParameterValue> {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        params
    }

    pub fn dynamics_info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for Dynamics {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        Dynamics::dynamics_info()
    }

    fn set_audio_parameters(&mut self, new_config: &AudioConfig) {
        self.envelope_detector = EnvelopeDetector::new(new_config.sample_rate);
        self.delay
            .borrow_mut()
            .change_sample_rate(new_config.sample_rate);
    }

    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;

        if param_idx == PARAM_ATTACK_TIME_MS {
            self.envelope_detector
                .set_attack_time_ms(param_value.as_flt());
        } else if param_idx == PARAM_RELEASE_TIME_MS {
            self.envelope_detector
                .set_release_time_ms(param_value.as_flt());
        } else if param_idx == PARAM_OUTPUT_GAIN_DB {
            self.real_output_gain = mathutils::db_to_linear(param_value.as_flt());
        } else if param_idx == PARAM_DELAY_MS {
            self.delay
                .borrow_mut()
                .set_delay_time_ms(param_value.as_flt(), true);
        }
    }

    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        let (read_buf, mut write_buf) = maybe_bufs.unwrap();
        let mut delay = self.delay.borrow_mut();
        for i in 0..num_samples {
            let x = delay.read_delayed_sample();

            let detect_db = self.envelope_detector.process(x);

            let mut fn_idx = self.processor_type.to_usize().unwrap();
            if self.params[PARAM_SOFT_KNEE].as_int() != 0 {
                fn_idx += 4;
            }

            let gain_db = GAIN_FNS[fn_idx](detect_db, &self.params);
            let gain_reduction_db = gain_db - detect_db;
            let gain_reduction = mathutils::db_to_linear(gain_reduction_db);

            delay.write_sample(read_buf.buf_read(i));
            write_buf.buf_write(i, x * gain_reduction * self.real_output_gain);
        }
    }
}

fn calculate_compressor_gain_hard_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();
    if detect_db <= threshold_db {
        return detect_db;
    }

    let ratio = params[PARAM_RATIO].as_flt();
    return threshold_db + (detect_db - threshold_db) / ratio;
}

fn calculate_limiter_gain_hard_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();

    return if detect_db <= threshold_db {
        detect_db
    } else {
        threshold_db
    };
}

fn calculate_expander_gain_hard_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();

    if detect_db >= threshold_db {
        return detect_db;
    }

    let ratio = params[PARAM_RATIO].as_flt();
    return threshold_db + ratio * (detect_db - threshold_db);
}

fn calculate_gate_gain_hard_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();

    if detect_db >= threshold_db {
        return detect_db;
    }

    return -96.0f32;
}

fn calculate_compressor_gain_soft_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();
    let knee_width = params[PARAM_KNEE_WIDTH_DB].as_flt();

    let detect_threshold_diff = detect_db - threshold_db;
    let abs_detect_threshold_diff = detect_threshold_diff.abs();

    let ratio = params[PARAM_RATIO].as_flt();
    return if 2.0f32 * detect_threshold_diff < -knee_width {
        detect_db
    } else if 2.0f32 * abs_detect_threshold_diff <= knee_width {
        detect_db
            + ((1.0f32 / ratio - 1.0f32)
                * (detect_threshold_diff + (knee_width / 2.0f32)).powf(2.0f32))
                / (2.0f32 * knee_width)
    } else {
        threshold_db + detect_threshold_diff / ratio
    };
}

fn calculate_limiter_gain_soft_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();
    let knee_width = params[PARAM_KNEE_WIDTH_DB].as_flt();

    let detect_threshold_diff = detect_db - threshold_db;
    let abs_detect_threshold_diff = detect_threshold_diff.abs();

    return if 2.0f32 * detect_threshold_diff < -knee_width {
        detect_db
    } else if 2.0f32 * abs_detect_threshold_diff <= knee_width {
        detect_db
            - (detect_threshold_diff + (knee_width / 2.0f32)).powf(2.0f32) / (2.0f32 * knee_width)
    } else {
        threshold_db
    };
}

fn calculate_expander_gain_soft_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();
    let knee_width = params[PARAM_KNEE_WIDTH_DB].as_flt();

    let detect_threshold_diff = detect_db - threshold_db;
    let abs_detect_threshold_diff = detect_threshold_diff.abs();

    let ratio = params[PARAM_RATIO].as_flt();
    return if 2.0f32 * detect_threshold_diff > knee_width {
        detect_db
    } else if 2.0f32 * abs_detect_threshold_diff > -knee_width {
        detect_db
            - ((1.0f32 / ratio) * (detect_threshold_diff - knee_width / 2.0f32).powf(2.0f32))
                / (2.0f32 * knee_width)
    } else {
        threshold_db + ratio * detect_threshold_diff
    };
}

fn calculate_gate_gain_soft_knee(
    detect_db: f32,
    params: &Vec<BoardEffectConfigParameterValue>,
) -> f32 {
    // mostly same as the soft-knee expander except for ratio
    let threshold_db = params[PARAM_THRESHOLD_DB].as_flt();
    let knee_width = params[PARAM_KNEE_WIDTH_DB].as_flt();

    let detect_threshold_diff = detect_db - threshold_db;
    let abs_detect_threshold_diff = detect_threshold_diff.abs();

    // TODO: see if this constant needs to be even bigger
    let ratio = params[PARAM_RATIO].as_flt() * 20.0f32;
    return if 2.0f32 * detect_threshold_diff > knee_width {
        detect_db
    } else if 2.0f32 * abs_detect_threshold_diff > -knee_width {
        detect_db
            - ((1.0f32 / ratio) * (detect_threshold_diff - knee_width / 2.0f32).powf(2.0f32))
                / (2.0f32 * knee_width)
    } else {
        threshold_db + ratio * detect_threshold_diff
    };
}
