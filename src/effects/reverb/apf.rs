use crate::utils::{
    delay_buf::DelayBuffer,
    lfo::{bipolar_to_unipolar, LowFrequencyOscillator},
    mathutils::lerp,
};

pub struct DelayAPF {
    lfo: LowFrequencyOscillator,
    lfo_depth: f32,
    lfo_max_modulation_ms: f32,

    delay_time_ms: f32,
    delay_buf: DelayBuffer,

    apf_g: f32,
    lpf_g: f32,
    lpf_state: f32,
}

impl DelayAPF {
    pub fn process(&mut self, x_n: f32) -> f32 {
        let min_delay = self.delay_time_ms;
        let max_delay = min_delay + self.lfo_max_modulation_ms;

        let modulated_delay = lerp(
            min_delay,
            max_delay,
            bipolar_to_unipolar(self.lfo.current_sample() * self.lfo_depth),
        );

        self.delay_buf.set_delay_time_ms(modulated_delay, true);
        let mut wn_D = self.delay_buf.read_delayed_sample();

        wn_D = wn_D * (1_f32 - self.lpf_g) + self.lpf_g * self.lpf_state;
        self.lpf_state = wn_D;

        // TODO:
        // This doesn't seem to exactly match the formula in the book:
        // y(n) = -g * x(n) + x(n - D) + g * y(n - D)
        let w_n = x_n + self.apf_g * wn_D;
        let y_n = -self.apf_g * w_n + wn_D;

        self.delay_buf.write_sample(y_n);

        y_n
    }
}
