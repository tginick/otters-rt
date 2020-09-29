use crate::conf::{
    AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue, ParameterRange,
};
use crate::context::BoardContext;
use crate::traits::AudioEffect;
use crate::utils::{
    biquad::{Biquad, BiquadCoefficients, IIRFilterType},
};

use crate::effects::basic_single_in_single_out;

use std::cell::RefCell;

const PARAMS: &'static [AdvertisedParameter] = &[
    AdvertisedParameter {
        name: "filter_type",
        range: ParameterRange::N(0, IIRFilterType::__NUM_IIR_FILTER_TYPES as i32),
        default_value: BoardEffectConfigParameterValue::N(IIRFilterType::FirstOrderLowPass as i32),
    },
    AdvertisedParameter {
        name: "corner_freq_hz",

        // Note that this value needs to be adjusted BASED ON SAMPLE RATE
        range: ParameterRange::F(0.0f32, 20480.0f32),
        default_value: BoardEffectConfigParameterValue::F(1024.0f32),
    },
    AdvertisedParameter {
        name: "boost_cut_db",
        range: ParameterRange::F(-20.0f32, 20.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.0f32),
    },
    AdvertisedParameter {
        name: "q",
        range: ParameterRange::F(0.707f32, 20.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.707f32),
    }
];

const PARAM_FILTER_TYPE: usize = 0;
const PARAM_CORNER_FREQ_HZ: usize = 1;
const PARAM_BOOST_CUT_DB: usize = 2;
const PARAM_Q: usize = 3;

pub struct BiquadFilter {
    params: Vec<BoardEffectConfigParameterValue>,
    biquad: RefCell<Biquad>,
}

impl BiquadFilter {
    pub fn new(ac: AudioConfig) -> BiquadFilter {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        BiquadFilter {
            params,
            biquad: RefCell::new(Biquad::new(BiquadCoefficients::first_order_lpf(
                PARAMS[PARAM_CORNER_FREQ_HZ].default_value.as_flt(),
                ac.sample_rate,
            ))),
        }
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for BiquadFilter {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        BiquadFilter::info()
    }

    fn set_audio_parameters(&mut self, new_config: &AudioConfig) {
        self.biquad.borrow_mut().change_sample_rate(new_config.sample_rate);
    }

    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;

        if param_idx == PARAM_CORNER_FREQ_HZ {
            self.biquad.borrow_mut().change_cutoff(param_value.as_flt());
        } else if param_idx == PARAM_FILTER_TYPE {
            self.biquad.borrow_mut().change_type(param_value.as_enum::<IIRFilterType>());
        } else if param_idx == PARAM_BOOST_CUT_DB {
            self.biquad.borrow_mut().change_shelf_gain(param_value.as_flt());
        } else if param_idx == PARAM_Q {
            self.biquad.borrow_mut().change_q(param_value.as_flt());
        }
    }

    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        let (read_buf, mut write_buf) = maybe_bufs.unwrap();
        let mut biquad = self.biquad.borrow_mut();

        for i in 0..num_samples {
            let sample = read_buf.buf_read(i);
            let filtered = biquad.filter(sample);
            write_buf.buf_write(i, filtered);
        }
    }
}
