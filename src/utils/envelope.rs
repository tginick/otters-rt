use super::mathutils;

use std::cell::Cell;

// TODO: when const fns stabilize, should be replaced with ln(0.368)
const ANALOG_RC_TIME_CONSTANT: f32 = -0.999672340813206123f32;

#[derive(PartialEq)]
pub enum EnvelopeDetectMode {
    Peak,
    MeanSquare,
    RootMeanSquare,
}

pub struct EnvelopeDetector {
    sample_rate: f32,
    pub detect_mode: EnvelopeDetectMode,
    last_envelope: Cell<f32>,
    pub should_clamp: bool,
    pub should_return_db: bool,
    attack_time_coefficient: f32,
    release_time_coefficient: f32,
}

impl EnvelopeDetector {
    pub fn new(sample_rate: f32) -> EnvelopeDetector {
        EnvelopeDetector {
            sample_rate,
            detect_mode: EnvelopeDetectMode::Peak,
            last_envelope: Cell::new(0.0f32),
            should_clamp: true,
            should_return_db: true,
            attack_time_coefficient: 0.0f32,
            release_time_coefficient: 0.0f32,
        }
    }

    pub fn set_attack_time_ms(&mut self, attack_time_ms: f32) {
        if attack_time_ms <= 0.0f32 {
            return;
        }

        self.attack_time_coefficient = mathutils::vexpf(
            ANALOG_RC_TIME_CONSTANT / (attack_time_ms * self.sample_rate * 0.001f32),
        );
    }

    pub fn set_release_time_ms(&mut self, release_time_ms: f32) {
        if release_time_ms <= 0.0f32 {
            return;
        }

        self.release_time_coefficient = mathutils::vexpf(
            ANALOG_RC_TIME_CONSTANT / (release_time_ms * self.sample_rate * 0.001f32),
        );
    }

    pub fn process(&self, x: f32) -> f32 {
        let mut abs_x = x.abs();

        if self.detect_mode == EnvelopeDetectMode::MeanSquare
            || self.detect_mode == EnvelopeDetectMode::RootMeanSquare
        {
            abs_x *= abs_x;
        }

        let last_envelope = self.last_envelope.get();
        let mut current_envelope = if abs_x > last_envelope {
            self.attack_time_coefficient * (last_envelope - abs_x) + abs_x
        } else {
            self.release_time_coefficient * (last_envelope - abs_x) + abs_x
        };

        if self.should_clamp {
            current_envelope = current_envelope.min(1.0f32);
        }

        current_envelope = current_envelope.max(0.0f32);

        self.last_envelope.set(current_envelope);

        if self.detect_mode == EnvelopeDetectMode::RootMeanSquare {
            current_envelope = mathutils::vsqrtf(current_envelope);
        }

        return if self.should_return_db {
            20.0f32 * current_envelope.log10()
        } else {
            current_envelope
        }
    }
}
