use crate::conf::{AdvertisedParameter, BoardEffectConfigParameterValue};
use crate::effects::VocoderContext;
use crate::traits::FrequencyDomainAudioEffect;
use crate::utils::fast_rand::WyHashPRNG;
use crate::utils::mathutils::{vcosf, vsinf, vsqrtf};
use fftw::array::AlignedVec;
use fftw::types::c32;
use std::time::SystemTime;

const PARAMS: &[AdvertisedParameter] = &[];

const RAND_MAX: u64 = 0x7fff;

pub struct Whisper {
    prng: WyHashPRNG
}

impl Whisper {
    pub fn new() -> Whisper {
        Whisper {
            prng: WyHashPRNG::new(SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
        }
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl FrequencyDomainAudioEffect for Whisper {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        Whisper::info()
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
            let amplitude = vsqrtf(fft[i].re * fft[i].re + fft[i].im * fft[i].im);
            let next_rand = (self.prng.next() % RAND_MAX) as f32; // we're not doing any crypto here
            let phase = (next_rand / RAND_MAX as f32) * crate::utils::TWO_PI;
            
            output[i] = c32::new(vcosf(phase) * amplitude, vsinf(phase) * amplitude);
        }
    }

    fn post_process(&self, _ifft: &mut AlignedVec<c32>) {}
}
