use super::VocoderContext;
use crate::conf::{AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue};
use crate::context::BoardContext;
use crate::effects::basic_single_in_single_out;
use crate::traits::{AudioEffect, FrequencyDomainAudioEffect};
use crate::utils::mathutils::vcosf;
use crate::utils::ringbuf::FFTCollectionBuffer;
use crate::utils::TWO_PI;
use fftw::array::AlignedVec;
use fftw::plan::*;
use fftw::types::*;

use std::cell::{Cell, RefCell};

#[derive(Clone, Copy)]
pub enum FFTWindowType {
    Hamming,
    Hann,
    BlackmanHarris,
}

struct FFTContext {
    forward_plan: C2CPlan32,
    backward_plan: C2CPlan32,
    fft_input_buf: AlignedVec<c32>,
    fft_output_buf: AlignedVec<c32>,
}

pub struct PhaseVocoder<T> {
    vocoder_context: VocoderContext,
    overlap_factor: f32,
    inv_gain_correction: f32,

    input_collection_buf: RefCell<FFTCollectionBuffer>,
    output_collection_buf: RefCell<FFTCollectionBuffer>,
    accumulated_sample_count: Cell<usize>,

    fft_context: RefCell<FFTContext>,

    freq_processor: T,
}

impl FFTContext {
    pub fn forward(&mut self) {
        self.forward_plan
            .c2c(&mut self.fft_input_buf, &mut self.fft_output_buf)
            .unwrap();
    }

    pub fn backward(&mut self) {
        self.backward_plan
            .c2c(&mut self.fft_input_buf, &mut self.fft_output_buf)
            .unwrap();
    }

    pub fn fft_buf<'a>(&'a mut self) -> &'a mut AlignedVec<c32> {
        &mut self.fft_input_buf
    }

    pub fn ifft_buf<'a>(&'a mut self) -> &'a mut AlignedVec<c32> {
        &mut self.fft_output_buf
    }

    pub fn both_bufs<'a>(&'a mut self) -> (&'a mut AlignedVec<c32>, &'a mut AlignedVec<c32>) {
        (&mut self.fft_input_buf, &mut self.fft_output_buf)
    }
}

impl<T: FrequencyDomainAudioEffect> PhaseVocoder<T> {
    pub fn new(
        frame_size: usize,
        hop_size: usize,
        window_type: FFTWindowType,
        freq_processor: T,
    ) -> PhaseVocoder<T> {
        // if hop size is 256 and frame size is 1024, this becomes 75%
        let overlap_factor = 1_f32 - ((hop_size as f32) / (frame_size as f32));
        let (window, inv_gain_correction) = create_window(window_type, overlap_factor, frame_size);

        let input_collection_buf = RefCell::new(FFTCollectionBuffer::new(frame_size << 2).unwrap());
        let output_collection_buf =
            RefCell::new(FFTCollectionBuffer::new(frame_size << 2).unwrap());
        {
            output_collection_buf.borrow_mut().set_write_idx(frame_size);
        }

        let forward_plan: C2CPlan32 =
            C2CPlan::aligned(&[frame_size], Sign::Forward, Flag::MEASURE).unwrap();
        let backward_plan: C2CPlan32 =
            C2CPlan::aligned(&[frame_size], Sign::Backward, Flag::MEASURE).unwrap();

        let mut fft_input_buf = AlignedVec::new(frame_size);
        let mut fft_output_buf = AlignedVec::new(frame_size);

        for i in 0..frame_size {
            fft_input_buf[i] = c32::new(0_f32, 0_f32);
            fft_output_buf[i] = c32::new(0_f32, 0_f32);
        }

        let fft_context = FFTContext {
            forward_plan,
            backward_plan,
            fft_input_buf,
            fft_output_buf,
        };

        let vocoder_context = VocoderContext {
            frame_size,
            hop_size,
            analysis_window: window,
        };

        PhaseVocoder {
            vocoder_context,
            overlap_factor,

            inv_gain_correction,

            input_collection_buf,
            output_collection_buf,
            accumulated_sample_count: Cell::new(0),

            fft_context: RefCell::new(fft_context),

            freq_processor,
        }
    }

    fn execute_one(&self, sample: f32) -> f32 {
        let mut input_collection_buf = self.input_collection_buf.borrow_mut();
        let mut output_collection_buf = self.output_collection_buf.borrow_mut();

        let current_output_read_idx = output_collection_buf.get_read_idx();
        let result = output_collection_buf.get_at_idx(current_output_read_idx);

        output_collection_buf.set_at_idx(current_output_read_idx, 0_f32);
        output_collection_buf.advance_read_idx();

        let current_input_write_idx = input_collection_buf.get_write_idx();
        input_collection_buf.set_at_idx(current_input_write_idx, sample);
        input_collection_buf.advance_write_idx();

        self.accumulated_sample_count
            .set(self.accumulated_sample_count.get() + 1);
        if self.accumulated_sample_count.get() == self.vocoder_context.frame_size {
            // time to do fft!
            let mut fft_context = self.fft_context.borrow_mut();
            for i in 0..self.vocoder_context.frame_size {
                let current_input_sample = input_collection_buf.get_at_read_idx();
                input_collection_buf.advance_read_idx();

                fft_context.fft_input_buf[i] = c32::new(
                    current_input_sample * self.vocoder_context.analysis_window[i],
                    0_f32,
                );
            }
            // overlap the read frames for fft
            input_collection_buf
                .rewind_read_idx(self.vocoder_context.frame_size - self.vocoder_context.hop_size);

            fft_context.forward();

            execute_freq_effect(&mut fft_context, &self.freq_processor);

            fft_context.backward();

            execute_post_processing(&mut fft_context, &self.freq_processor);

            // overlap add
            overlap_add(
                &mut fft_context,
                self.vocoder_context.frame_size,
                &mut output_collection_buf,
                self.inv_gain_correction,
            );

            output_collection_buf
                .rewind_write_idx(self.vocoder_context.frame_size - self.vocoder_context.hop_size);

            // update sample counter
            self.accumulated_sample_count
                .set(self.accumulated_sample_count.get() - self.vocoder_context.hop_size);
        }

        result
    }
}

impl<T: FrequencyDomainAudioEffect> AudioEffect for PhaseVocoder<T> {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        self.freq_processor.advertise_parameters()
    }

    fn set_audio_parameters(&mut self, _new_config: &AudioConfig) {}

    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.freq_processor
            .set_effect_parameter(param_idx, param_value);
    }

    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        let (read_buf, mut write_buf) = maybe_bufs.unwrap();

        for i in 0..num_samples {
            let current_sample = read_buf.buf_read(i);

            let next_sample = self.execute_one(current_sample);

            write_buf.buf_write(i, next_sample);
        }
    }
}

pub fn create_window(
    window_type: FFTWindowType,
    overlap_pct: f32,
    frame_size: usize,
) -> (AlignedVec<f32>, f32) {
    let mut r = AlignedVec::new(frame_size);
    for i in 0..frame_size {
        let n = i as f32;
        r[i] = match window_type {
            FFTWindowType::Hamming => {
                0.54_f32 - 0.46_f32 * vcosf((n * TWO_PI) / (frame_size as f32))
            }
            FFTWindowType::Hann => 0.5_f32 - (1.0_f32 - vcosf((n * TWO_PI) / (frame_size as f32))),
            FFTWindowType::BlackmanHarris => {
                0.42323_f32 - (0.49755_f32 * vcosf((n * TWO_PI) / (frame_size as f32)))
                    + 0.07922_f32 * vcosf((n * TWO_PI) / (frame_size as f32))
            }
        }
    }

    let inv_gain_correction = r.iter().fold(0.0f32, |acc, x| acc + x);
    (r, (1.0f32 - overlap_pct) / inv_gain_correction)
}

fn execute_freq_effect<T: FrequencyDomainAudioEffect>(
    fft_context: &mut FFTContext,
    freq_processor: &T,
) {
    // output_buf contains the fft
    let (mut input_buf, output_buf) = fft_context.both_bufs();

    freq_processor.execute(&output_buf, &mut input_buf);
}

fn execute_post_processing<T: FrequencyDomainAudioEffect>(
    fft_context: &mut FFTContext,
    freq_effect: &T,
) {
    let mut output_buf = fft_context.ifft_buf();
    freq_effect.post_process(&mut output_buf);
}

fn overlap_add(
    fft_context: &mut FFTContext,
    frame_size: usize,
    output_collection_buf: &mut FFTCollectionBuffer,
    inv_gain_correction: f32,
) {
    let output_buf = fft_context.ifft_buf();

    for i in 0..frame_size {
        let current_sample =
            output_collection_buf.get_at_idx(output_collection_buf.get_write_idx());
        output_collection_buf
            .set_at_write_idx(output_buf[i].re * inv_gain_correction + current_sample);
        output_collection_buf.advance_write_idx();
    }
}
