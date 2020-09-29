use super::mathutils;
use super::ringbuf::SimpleFloatBuffer;
use crate::consts;

pub struct DelayBuffer {
    buf: SimpleFloatBuffer,
    sample_rate: f32,
    max_delay_ms: f32,
    delay_time_ms: f32,
    whole_delay_time_samples: i32,
    fract_delay_time_samples: f32,
}

impl DelayBuffer {
    pub fn with_sample_rate(sample_rate: f32) -> DelayBuffer {
        DelayBuffer::with_sample_rate_and_max_delay(sample_rate, consts::MAX_DELAY_MS)
    }

    pub fn with_sample_rate_and_max_delay(sample_rate: f32, max_delay_ms: f32) -> DelayBuffer {
        DelayBuffer {
            buf: SimpleFloatBuffer::with_max_capacity(
                (sample_rate * max_delay_ms / 1000.0f32) as usize,
            ),
            sample_rate,
            max_delay_ms,
            delay_time_ms: 0f32,
            whole_delay_time_samples: 0,
            fract_delay_time_samples: 0f32,
        }
    }

    pub fn change_sample_rate(&mut self, new_sample_rate: f32) {
        self.sample_rate = new_sample_rate;
        self.buf = SimpleFloatBuffer::with_max_capacity(
            (self.sample_rate * self.max_delay_ms / 1000.0f32) as usize,
        );

        self.set_delay_time_ms(self.delay_time_ms, true);
    }

    pub fn set_delay_time_ms(&mut self, delay_time_ms: f32, should_clamp_if_high: bool) {
        if delay_time_ms < 0.0f32 {
            return;
        }

        let mut real_delay_time = delay_time_ms * self.sample_rate / 1000.0f32;

        if real_delay_time as usize >= self.buf.get_capacity() {
            if should_clamp_if_high {
                real_delay_time = (self.buf.get_capacity() - 1) as f32;
            } else {
                return;
            }
        }

        self.delay_time_ms = delay_time_ms;

        let (ipart, fpart) = mathutils::vmodf(real_delay_time);

        self.fract_delay_time_samples = fpart;
        self.whole_delay_time_samples = num::clamp(ipart, 0, self.buf.get_capacity() as i32);

        self.clamp_delay_sample_count();
    }

    pub fn set_delay_sample_count_directly(&mut self, whole_delay_sample_count: i32, fract_delay_sample_count: f32) {
        self.whole_delay_time_samples = whole_delay_sample_count;
        self.fract_delay_time_samples = fract_delay_sample_count;

        self.clamp_delay_sample_count();
    }

    pub fn get_delay_sample_count(&self) -> f32 {
        return (self.whole_delay_time_samples as f32) + self.fract_delay_time_samples;
    }

    pub fn get_sample_rate(&self) -> f32 {
        return self.sample_rate;
    }

    pub fn read_delayed_sample(&self) -> f32 {
        let sample_1 = self
            .buf
            .read(self.buf.get_limit() - self.whole_delay_time_samples as usize - 1);
        let sample_2 = self
            .buf
            .read(self.buf.get_limit() - self.whole_delay_time_samples as usize - 2);

        mathutils::lerp(sample_1, sample_2, self.fract_delay_time_samples)
    }

    pub fn write_sample(&mut self, sample: f32) {
        self.buf.write(sample);
    }

    fn clamp_delay_sample_count(&mut self) {
        if self.whole_delay_time_samples == self.buf.get_capacity() as i32 - 1 {
            self.whole_delay_time_samples = self.buf.get_capacity() as i32 - 2;
            self.fract_delay_time_samples = mathutils::nextafter(1.0f32, 0.0f32);
        }
    }
}
