use crate::conf::{
    AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue, ParameterRange,
};
use crate::effects::basic_single_in_single_out;
use crate::traits::AudioEffect;
use crate::utils::{
    biquad::{Biquad, BiquadCoefficients},
    lfo::{LFOWaveForm, LowFrequencyOscillator},
    mathutils::bipolar_lerp,
};
use std::cell::RefCell;

const PARAMS: &'static [AdvertisedParameter] = &[
    AdvertisedParameter {
        name: "mod_rate_hz",
        range: ParameterRange::F(0.02f32, 10.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.5f32),
    },
    AdvertisedParameter {
        name: "depth_pct",
        range: ParameterRange::F(0.0f32, 1.0f32),
        default_value: BoardEffectConfigParameterValue::F(0.5f32),
    },
    AdvertisedParameter {
        name: "intensity_pct",
        range: ParameterRange::F(0.0f32, 0.99f32),
        default_value: BoardEffectConfigParameterValue::F(0.5f32),
    },
];

const PARAM_MOD_RATE_HZ: usize = 0;
const PARAM_DEPTH_PCT: usize = 1;
const PARAM_INTENSITY_PCT: usize = 2;

struct ModulatedAPF {
    min_freq: f32,
    max_freq: f32,
    sample_rate: f32,
    filter: Biquad,
}

pub struct MonoPhaser {
    params: Vec<BoardEffectConfigParameterValue>,

    apfs: RefCell<Vec<ModulatedAPF>>,
    lfo: RefCell<LowFrequencyOscillator>,
}

impl ModulatedAPF {
    fn new(min_freq: f32, max_freq: f32, sample_rate: f32) -> ModulatedAPF {
        ModulatedAPF {
            min_freq,
            max_freq,
            sample_rate,
            filter: Biquad::new(BiquadCoefficients::first_order_apf(min_freq, sample_rate)),
        }
    }

    fn update_cutoff(&mut self, lfo_sample: f32) {
        // lfo sample is [-1, 1]
        let new_freq = bipolar_lerp(self.min_freq, self.max_freq, lfo_sample);
        self.filter
            .change_params(BiquadCoefficients::first_order_apf(
                new_freq,
                self.sample_rate,
            ));
    }

    fn execute_filter(&mut self, input: f32) -> f32 {
        self.filter.filter(input)
    }

    fn g(&self) -> f32 {
        self.filter.g()
    }

    fn s(&self) -> f32 {
        self.filter.s()
    }
}

impl MonoPhaser {
    pub fn new(ac: AudioConfig) -> MonoPhaser {
        let apfs = RefCell::new(vec![
            ModulatedAPF::new(32.0f32, 1500.0f32, ac.sample_rate),
            ModulatedAPF::new(68.0f32, 3400.0f32, ac.sample_rate),
            ModulatedAPF::new(96.0f32, 4800.0f32, ac.sample_rate),
            ModulatedAPF::new(212.0f32, 10000.0f32, ac.sample_rate),
            ModulatedAPF::new(320.0f32, 16000.0f32, ac.sample_rate),
            ModulatedAPF::new(636.0f32, 20400.0f32, ac.sample_rate),
        ]);

        let lfo = RefCell::new(LowFrequencyOscillator::new(
            LFOWaveForm::Sine,
            PARAMS[PARAM_MOD_RATE_HZ].default_value.as_flt(),
            ac.sample_rate,
        ));

        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        MonoPhaser { params, apfs, lfo }
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for MonoPhaser {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        MonoPhaser::info()
    }

    fn set_audio_parameters(&mut self, new_config: &AudioConfig) {
        self.lfo
            .borrow_mut()
            .change_sample_rate(new_config.sample_rate);

        for apf in self.apfs.borrow_mut().iter_mut() {
            apf.filter.change_sample_rate(new_config.sample_rate);
        }
    }

    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;
        if param_idx == PARAM_MOD_RATE_HZ {
            self.lfo
                .borrow_mut()
                .change_oscillation_freq(self.params[PARAM_MOD_RATE_HZ].as_flt());
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
        let (read_buf, mut write_buf) = maybe_bufs.unwrap();
        let mut apfs = self.apfs.borrow_mut();
        let mut lfo = self.lfo.borrow_mut();
        for i in 0..num_samples {
            let current_lfo_sample = lfo.current_sample();
            for apf in apfs.iter_mut() {
                apf.update_cutoff(current_lfo_sample * self.params[PARAM_DEPTH_PCT].as_flt());
            }
            lfo.oscillate();

            let gamma_1 = apfs[5].g();
            let gamma_2 = apfs[4].g() * gamma_1;
            let gamma_3 = apfs[3].g() * gamma_2;
            let gamma_4 = apfs[2].g() * gamma_3;
            let gamma_5 = apfs[1].g() * gamma_4;
            let gamma_6 = apfs[0].g() * gamma_5;

            let k = self.params[PARAM_INTENSITY_PCT].as_flt();
            let alpha0 = 1.0f32 / (1.0f32 + k * gamma_6);

            let s_n = gamma_5 * apfs[0].s()
                + gamma_4 * apfs[1].s()
                + gamma_3 * apfs[2].s()
                + gamma_2 * apfs[3].s()
                + gamma_1 * apfs[4].s()
                + apfs[5].s();

            let x_n = read_buf.buf_read(i);
            let mut u = alpha0 * (x_n + k * s_n);
            for apf in apfs.iter_mut() {
                u = apf.execute_filter(u);
            }

            write_buf.buf_write(i, 0.125_f32 * x_n + 1.25_f32 * u);
        }
    }
}
