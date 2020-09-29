use crate::conf::{AdvertisedParameter, AudioConfig, BoardEffectConfigParameterValue};
use crate::context::BoardContext;
use crate::effects::VocoderContext;
use fftw::array::AlignedVec;
use fftw::types::c32;

pub trait AudioEffect {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter];
    fn set_audio_parameters(&mut self, new_config: &AudioConfig);
    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    );
    fn execute(&self, context: &BoardContext, connection_idx: usize, num_samples: usize);
}

pub trait FrequencyDomainAudioEffect {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter];
    fn post_initialize(&mut self, vocoder_context: &VocoderContext);
    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    );
    fn execute(&self, fft: &AlignedVec<c32>, output: &mut AlignedVec<c32>);
    fn post_process(&self, ifft: &mut AlignedVec<c32>);
}