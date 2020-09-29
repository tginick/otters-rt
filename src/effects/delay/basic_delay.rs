use crate::conf::{
    AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue, ParameterRange,
};
use crate::consts;
use crate::context::BoardContext;
use crate::effects::basic_single_in_single_out;
use crate::traits::AudioEffect;
use crate::utils::delay_buf::DelayBuffer;

use std::cell::RefCell;

const BASIC_PARAMS: &'static [AdvertisedParameter] = &[
    AdvertisedParameter {
        name: "delay_time_ms",
        range: ParameterRange::F(0.0f32, consts::MAX_DELAY_MS),
        default_value: BoardEffectConfigParameterValue::F(1000.0f32),
    },
    AdvertisedParameter {
        name: "feedback_pct",
        range: ParameterRange::F(-1.0f32, 1.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.0f32),
    },
    AdvertisedParameter {
        name: "wet_dry_pct",
        range: ParameterRange::F(0.0f32, 1.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.5f32),
    },
];

const PARAM_DELAY_TIME_MS: usize = 0;
const PARAM_FEEDBACK_PCT: usize = 1;
const PARAM_WET_DRY_PCT: usize = 2;

pub struct MonoDelayBasic {
    params: Vec<BoardEffectConfigParameterValue>,

    delay_buf: RefCell<DelayBuffer>,
}

impl MonoDelayBasic {
    pub fn info() -> &'static [AdvertisedParameter] {
        BASIC_PARAMS
    }

    pub fn new(ac: AudioConfig) -> MonoDelayBasic {
        let mut params = Vec::with_capacity(BASIC_PARAMS.len());
        for i in 0..BASIC_PARAMS.len() {
            params.push(BASIC_PARAMS[i].default_value);
        }

        let delay_buf = RefCell::new(DelayBuffer::with_sample_rate(ac.sample_rate));

        MonoDelayBasic { params, delay_buf }
    }
}

impl AudioEffect for MonoDelayBasic {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        MonoDelayBasic::info()
    }

    fn set_audio_parameters(&mut self, new_config: &AudioConfig) {
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

        if param_idx == PARAM_DELAY_TIME_MS {
            self.delay_buf
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

        let wetness = self.params[PARAM_WET_DRY_PCT].as_flt();
        let dryness = 1.0f32 - wetness;
        let feedback = self.params[PARAM_FEEDBACK_PCT].as_flt();

        let mut delay_ref = self.delay_buf.borrow_mut();
        for i in 0..num_samples {
            let xn = read_buf.buf_read(i);
            let yn = delay_ref.read_delayed_sample();
            let dn = xn + feedback * yn;

            delay_ref.write_sample(dn);

            let on = dryness * xn + wetness * yn;
            write_buf.buf_write(i, on);

            /*
            // old implementation
            let interp_sample = delay_ref.read_delayed_sample();
            let interp_output = output_ref.read_delayed_sample();

            let current_sample = read_buf.buf_read(i);
            let y = dryness * current_sample + wetness * (interp_sample + feedback * interp_output);

            write_buf.buf_write(i, y);

            delay_ref.write_sample(current_sample);
            output_ref.write_sample(y);
            */
        }
    }
}
