use crate::utils::delay_buf::DelayBuffer;

use std::cell::RefCell;

struct LPFCombFilter {
    delay_buf: DelayBuffer,
    comb_g: f32,
    lpf_g: f32, // [0, 0.9999]
    lpf_state: f32,
    rt60_ms: f32,
}

impl LPFCombFilter {
    pub fn new(delay_time_ms: f32, sample_rate: f32, rt60_ms: f32, lpf_g: f32) -> LPFCombFilter {
        let mut delay_buf = DelayBuffer::with_sample_rate(sample_rate);
        delay_buf.set_delay_time_ms(delay_time_ms, true);

        let delay_sample_count = delay_buf.get_delay_sample_count();

        let comb_gain = calculate_comb_gain(delay_sample_count, sample_rate, rt60_ms);

        LPFCombFilter {
            delay_buf,
            comb_g: comb_gain,
            lpf_g,
            lpf_state: 0_f32,
            rt60_ms,
        }
    }

    pub fn change_sample_rate(&mut self, new_sample_rate: f32) {
        self.delay_buf.change_sample_rate(new_sample_rate);

        // update comb gain
        self.comb_g = calculate_comb_gain(
            self.delay_buf.get_delay_sample_count(),
            new_sample_rate,
            self.rt60_ms,
        );

        // reset lpf
        self.lpf_state = 0_f32;
    }

    pub fn change_delay_time(&mut self, new_delay_time: f32) {
        self.delay_buf.set_delay_time_ms(new_delay_time, true);

        // update comb gain
        self.comb_g = calculate_comb_gain(
            self.delay_buf.get_delay_sample_count(),
            self.delay_buf.get_sample_rate(),
            self.rt60_ms,
        );

        // reset lpf
        self.lpf_state = 0_f32;
    }

    pub fn set_lpf_g(&mut self, new_lpf_g: f32) {
        self.lpf_g = new_lpf_g;
    }

    pub fn set_rt60_ms(&mut self, rt60_ms: f32) {
        let new_g = calculate_comb_gain(
            self.delay_buf.get_delay_sample_count(),
            self.delay_buf.get_sample_rate(),
            rt60_ms,
        );
        self.comb_g = new_g;
    }

    pub fn set_comb_g_directly(&mut self, new_comb_g: f32) {
        self.comb_g = new_comb_g;
    }

    pub fn process(&mut self, x_n: f32) -> f32 {
        let y_n = self.delay_buf.read_delayed_sample();

        let g2 = self.lpf_g * (1_f32 - self.comb_g);
        let lpf_sample = y_n + g2 * self.lpf_state;

        let delay_input = x_n + self.comb_g * lpf_sample;
        self.lpf_state = lpf_sample;

        self.delay_buf.write_sample(delay_input);

        y_n
    }
}

fn calculate_comb_gain(delay_sample_count: f32, sample_rate: f32, rt60_ms: f32) -> f32 {
    let exponent = -3_f32 * delay_sample_count / sample_rate;
    let rt60_s = rt60_ms / 1000_f32;

    10_f32.powf(exponent / rt60_s)
}
