use crate::conf::{AdvertisedParameter, BoardEffectConfigParameterValue};
use crate::effects::VocoderContext;
use crate::traits::FrequencyDomainAudioEffect;
use crate::utils::mathutils::vsqrtf;
use fftw::array::AlignedVec;
use fftw::types::c32;

const PARAMS: &[AdvertisedParameter] = &[];

pub struct Robotize {}

impl Robotize {
    pub fn new() -> Robotize {
        Robotize {}
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl FrequencyDomainAudioEffect for Robotize {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        Robotize::info()
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
            let re = vsqrtf(fft[i].re * fft[i].re + fft[i].im * fft[i].im);

            output[i] = c32::new(re, 0.0f32);
        }
    }

    fn post_process(&self, _ifft: &mut AlignedVec<c32>) {}
}
