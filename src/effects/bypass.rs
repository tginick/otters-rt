use crate::conf::{AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue};
use crate::context::BoardContext;
use crate::effects::{basic_single_in_single_out, VocoderContext};
use crate::traits::{AudioEffect, FrequencyDomainAudioEffect};

use fftw::array::AlignedVec;
use fftw::types::c32;

const PARAMS: &'static [AdvertisedParameter] = &[];

pub struct MonoBypass {}

pub struct GenericBypass {}

// Mostly just used to make sure the vocoder implementation works
pub struct VocoderBypass {}

impl MonoBypass {
    pub fn new() -> MonoBypass {
        MonoBypass {}
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl GenericBypass {
    pub fn new() -> GenericBypass {
        GenericBypass {}
    }
}

impl VocoderBypass {
    pub fn new() -> VocoderBypass {
        VocoderBypass {}
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for MonoBypass {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        return PARAMS;
    }

    fn set_audio_parameters(&mut self, _new_config: &AudioConfig) {}

    fn set_effect_parameter(&mut self, _param_idx: usize, _param: BoardEffectConfigParameterValue) {
    }

    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        let (read_buf, mut write_buf) = maybe_bufs.unwrap();

        for i in 0..num_samples {
            let r = read_buf.buf_read(i);
            write_buf.buf_write(i, r);
        }
    }
}

impl AudioEffect for GenericBypass {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        PARAMS
    }

    fn set_audio_parameters(&mut self, _new_config: &AudioConfig) {}

    fn set_effect_parameter(
        &mut self,
        _param_idx: usize,
        _param_value: BoardEffectConfigParameterValue,
    ) {
    }

    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize) {
        let inputs = context.get_inputs_for_connection(connection_idx);
        let outputs = context.get_outputs_for_connection(connection_idx);

        let min_end = inputs.len().min(outputs.len());
        for i in 0..min_end {
            let read_buf = context.get_buffer_for_read(inputs[i]);
            let mut write_buf = context.get_buffer_for_write(outputs[i]);

            for j in 0..num_samples {
                write_buf.buf_write(j, read_buf.buf_read(j));
            }
        }

        // # outputs > # inputs
        // write 0 to extra outputs
        if inputs.len() == min_end {
            for i in min_end..outputs.len() {
                let mut write_buf = context.get_buffer_for_write(i);

                for j in 0..num_samples {
                    write_buf.buf_write(j, 0.0f32);
                }
            }
        }

        // # outputs < # inputs
        // do nothing!
    }
}

impl FrequencyDomainAudioEffect for VocoderBypass {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        PARAMS
    }

    fn post_initialize(&mut self, _vocoder_context: &VocoderContext) {}

    fn set_effect_parameter(
        &mut self,
        _param_idx: usize,
        _param_value: BoardEffectConfigParameterValue,
    ) {
    }

    fn execute(&self, fft: &AlignedVec<c32>, output: &mut AlignedVec<c32>) {
        for i in 0..fft.len() {
            output[i] = fft[i];
        }
    }

    fn post_process(&self, _ifft: &mut AlignedVec<c32>) {}
}
