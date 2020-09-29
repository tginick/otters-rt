use num_derive::FromPrimitive;

#[derive(FromPrimitive)]
pub enum LFOWaveForm {
    Triangle = 0,
    Sine,
    Sawtooth,
}

impl Default for LFOWaveForm {
    fn default() -> Self {
        LFOWaveForm::Sine
    }
}

pub struct LowFrequencyOscillator {
    modulo_counter: f32,
    modulo_inc: f32,

    oscillation_freq: f32,
    sample_rate: f32,
    waveform: LFOWaveForm,
}

impl LowFrequencyOscillator {
    pub fn new(
        waveform: LFOWaveForm,
        oscillation_freq: f32,
        sample_rate: f32,
    ) -> LowFrequencyOscillator {
        LowFrequencyOscillator {
            modulo_counter: 0.0f32,
            modulo_inc: oscillation_freq / sample_rate,
            oscillation_freq,
            sample_rate,
            waveform,
        }
    }

    pub fn change_oscillation_freq(&mut self, new_freq: f32) {
        self.oscillation_freq = new_freq;
        self.modulo_inc = self.oscillation_freq / self.sample_rate;
    }

    pub fn change_sample_rate(&mut self, new_sample_rate: f32) {
        self.sample_rate = new_sample_rate;
        self.modulo_inc = self.oscillation_freq / new_sample_rate;
        self.modulo_counter = 0.0f32;
    }

    pub fn oscillate(&mut self) {
        self.modulo_counter += self.modulo_inc;
        if self.modulo_counter >= 1.0f32 {
            self.modulo_counter -= 1.0f32;
        }
    }

    pub fn current_sample(&mut self) -> f32 {
        match self.waveform {
            LFOWaveForm::Triangle => triangle_wave(self.modulo_counter),
            LFOWaveForm::Sawtooth => sawtooth_wave(self.modulo_counter),
            LFOWaveForm::Sine => sine_wave(self.modulo_counter),
        }
    }
}

// used to convert bipolar lfo values to unipolar
pub fn bipolar_to_unipolar(v: f32) -> f32 {
    (v + 1.0f32) / 2.0f32
}

const B: f32 = 4.0f32 / std::f32::consts::PI;
const C: f32 = -4.0f32 / (std::f32::consts::PI * std::f32::consts::PI);
const P: f32 = 0.225f32;
fn parabolic_sine(phase: f32) -> f32 {
    let mut y = B * phase + C * phase * phase.abs();
    y = P * (y * y.abs() - y) + y;
    y
}

fn sine_wave(v: f32) -> f32 {
    let angle = v * super::TWO_PI - std::f32::consts::PI;
    parabolic_sine(-angle)
}

fn triangle_wave(v: f32) -> f32 {
    // unipolar to bipolar
    let bipolar_v = 2.0f32 * v - 1.0f32;

    2.0f32 * bipolar_v.abs() - 1.0f32
}

fn sawtooth_wave(v: f32) -> f32 {
    // just convert to bipolar
    2.0f32 * v - 1.0f32
}
