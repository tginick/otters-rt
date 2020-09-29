use crate::conf::{
    AdvertisedParameter, BoardEffectConfigParameterValue, ParameterRange,
};
use crate::effects::VocoderContext;
use crate::utils::mathutils::{vcosf, vsinf};
use crate::traits::FrequencyDomainAudioEffect;
use fftw::array::AlignedVec;
use fftw::types::c32;

use std::cell::Cell;

const PARAMS: &[AdvertisedParameter] = &[
    // TODO: maybe support microtones in the future
    AdvertisedParameter {
        name: "semitone_difference",
        range: ParameterRange::N(-12, 12), // [-12, 12] => down 1 or up 1 octave
        default_value: BoardEffectConfigParameterValue::N(0),
    },
];

const PARAM_SEMITONE_DIFFERENCE: usize = 0;

const FRAME_SIZE: usize = 1024; // must be a power of 2. higher for better results
const OVERLAP_PCT: f32 = 0.75f32;

struct OceanPitchShifterExtraParams {
    overlap_factor: usize,
    overlap_factor_sq: usize,
    output_hop_index: Cell<isize>,
    frame_size: usize,
    hop_size: usize,
    num_input_bins: usize,
    num_output_bins: usize,

    // note we're not zero padding right now so this is always 1
    zero_pad_factor: usize,

    // a copy of the analysis window from the vocoder
    copied_window: AlignedVec<f32>,

    unity_roots: AlignedVec<c32>,
}

pub struct OceanPitchShifter {
    params: Vec<BoardEffectConfigParameterValue>,
    frequency_multiplier: f32,

    extra_params: Option<OceanPitchShifterExtraParams>,
}

impl OceanPitchShifter {
    pub fn new() -> OceanPitchShifter {
        let mut params = Vec::new();
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        let frequency_multiplier = semitones_to_freq(params[PARAM_SEMITONE_DIFFERENCE].as_int());
        OceanPitchShifter {
            params,
            frequency_multiplier,

            extra_params: None,
        }
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl FrequencyDomainAudioEffect for OceanPitchShifter {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        OceanPitchShifter::info()
    }

    fn post_initialize(&mut self, vocoder_context: &VocoderContext) {
        let overlap_factor = vocoder_context.frame_size / vocoder_context.hop_size;
        let overlap_factor_sq = 1_usize << overlap_factor;

        // lifted from reference java implementation by Nicolas Juillerat
        /*
         * Running index of the STFT frame being processed (in the frequency domain) since the beginning.
         * Because we need to accumulate (overlap - 1) input hops before the first hop actually gets
         * at the begining of the FFT'ed block, we have a latency of (overlap - 1) hops and hence start with a
         * corresponding negative value:
         */
        self.extra_params = Some(OceanPitchShifterExtraParams {
            overlap_factor,
            overlap_factor_sq,
            output_hop_index: Cell::new(-(overlap_factor as isize) - 1),
            hop_size: vocoder_context.hop_size,
            frame_size: vocoder_context.frame_size,
            zero_pad_factor: 1,

            num_input_bins: vocoder_context.frame_size / 2 + 1,
            num_output_bins: vocoder_context.frame_size * 1 / 2 + 1, // * 1 is zero pad factor, which is 1 for our use case
            
            copied_window: vocoder_context.analysis_window.clone(),
            unity_roots: generate_unity_roots((overlap_factor_sq * 1) as isize), // same as above re: zero pad
        });
    }

    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;

        if param_idx == PARAM_SEMITONE_DIFFERENCE {
            self.frequency_multiplier = semitones_to_freq(param_value.as_int());
        }
    }

    fn execute(&self, fft: &AlignedVec<c32>, output: &mut AlignedVec<c32>) {
        if self.extra_params.is_none() {
            return;
        }

        let extra_params = self.extra_params.as_ref().unwrap();

        output[0] = fft[0];
        for i in 1..fft.len() {
            output[i] = c32::new(0_f32, 0_f32);
        }

        let cycle_length = extra_params.overlap_factor_sq * extra_params.zero_pad_factor;
        let cycle_idx = (extra_params.output_hop_index.get() + (cycle_length as isize) * 2) % (cycle_length as isize);
        let cycle_idx = cycle_idx as usize;

        for src_bin_idx in 1..extra_params.num_input_bins {
            let padded_src_bin_idx = src_bin_idx * extra_params.zero_pad_factor;

            let dst_bin_idx = (padded_src_bin_idx as f32 * self.frequency_multiplier + 0.5_f32) as usize;

            if dst_bin_idx <= 0 || dst_bin_idx >= extra_params.num_output_bins {
                continue;
            }

            let mut work = fft[src_bin_idx];

            let cycle_shift = if dst_bin_idx >= padded_src_bin_idx {
                (dst_bin_idx - padded_src_bin_idx) as usize % cycle_length
            } else {
                cycle_length - (padded_src_bin_idx - dst_bin_idx) as usize % cycle_length
            };

            let phase_shift = (cycle_idx * cycle_shift) % cycle_length;
            if phase_shift != 0 {
                work *= extra_params.unity_roots[(cycle_length - phase_shift) % cycle_length];
            }

            output[dst_bin_idx] += work;
        }

        extra_params
            .output_hop_index
            .set(extra_params.output_hop_index.get() + 1);
    }

    fn post_process(&self, ifft: &mut AlignedVec<c32>) {
        if self.extra_params.is_none() {
            return;
        }

        let extra_params = self.extra_params.as_ref().unwrap();
        for i in 0..extra_params.hop_size {
            ifft[i].re = ifft[i].re
                * sample_demodulation_window(
                    &extra_params.copied_window,
                    i,
                    extra_params.output_hop_index.get(),
                    extra_params.frame_size,
                    extra_params.overlap_factor,
                    extra_params.zero_pad_factor,
                    self.frequency_multiplier,
                );
        }
    }
}

fn semitones_to_freq(semitones: i32) -> f32 {
    2.0f32.powf((semitones as f32) / 12.0f32)
}

fn generate_unity_roots(cycle_length: isize) -> AlignedVec<c32> {
    if cycle_length <= 0 {
        return AlignedVec::new(1);
    }

    let cycle_length = cycle_length as usize;
    let mut roots = AlignedVec::new(cycle_length);

    let cos_inc = vcosf(crate::utils::TWO_PI / cycle_length as f32);
    let sin_inc = vsinf(crate::utils::TWO_PI / cycle_length as f32);

    roots[0] = c32::new(1.0_f32, 0.0_f32);

    let mut lre = 1.0_f32;
    let mut lim = 0.0_f32;

    for i in 1..cycle_length as usize {
        let re = cos_inc * lre - sin_inc * lim;
        let im = sin_inc * lre + cos_inc * lim;

        lre = re;
        lim = im;

        roots[i] = c32::new(re, im);
    }

    roots
}

// TODO: try to store the result of this function as the window is periodic
/* Original comment by Nicolas Juillerat
* For given parameters, the demodulation window has a period of <tt>overlap * zeroPad</tt> and
* could in theory be computed once and stored in an array. Here we compute it on the fly, one sample
* at a time.
*/
fn sample_demodulation_window(
    original_analysis_window: &AlignedVec<f32>,
    frame_idx: usize,
    hop_idx: isize,
    frame_size: usize,
    overlap_factor: usize,
    zero_pad_factor: usize,
    frequency_multiplier: f32,
) -> f32 {
    let mut r = 0_f32;
    let hop_size = frame_size / overlap_factor;
    for k in 0..overlap_factor {
        let offset = k * hop_size + frame_idx;
        r += sample_modified_analysis_window(
            original_analysis_window,
            offset,
            hop_idx - k as isize,
            frame_size,
            overlap_factor,
            zero_pad_factor,
            frequency_multiplier,
        ) * original_analysis_window[offset];
    }

    const THRESHOLD: f32 = 0.1f32;
    if r <= THRESHOLD {
        return 1.0f32 / THRESHOLD;
    }

    return 1.0f32 / r;
}

fn sample_modified_analysis_window(
    original_analysis_window: &AlignedVec<f32>,
    frame_idx: usize,
    hop_idx: isize,
    frame_size: usize,
    overlap_factor: usize,
    zero_pad_factor: usize,
    frequency_multiplier: f32,
) -> f32 {
    let padded_freq_multiplier = frequency_multiplier * (zero_pad_factor as f32);
    let floored_freq_multiplier = padded_freq_multiplier.floor() as i32;
    let ceil_freq_multiplier = floored_freq_multiplier + 1;

    let dist_from_ceil = padded_freq_multiplier - (floored_freq_multiplier as f32);
    let dist_from_floor = 1.0_f32 - dist_from_ceil;

    let floor_psr_window = sample_modified_analysis_window_int_psr(
        original_analysis_window,
        frame_idx,
        hop_idx,
        frame_size,
        overlap_factor,
        zero_pad_factor,
        floored_freq_multiplier,
    );
    let ceil_psr_window = sample_modified_analysis_window_int_psr(
        original_analysis_window,
        frame_idx,
        hop_idx,
        frame_size,
        overlap_factor,
        zero_pad_factor,
        ceil_freq_multiplier,
    );

    floor_psr_window * dist_from_floor + ceil_psr_window * dist_from_ceil
}

fn sample_modified_analysis_window_int_psr(
    original_analysis_window: &AlignedVec<f32>,
    frame_idx: usize,
    hop_idx: isize,
    frame_size: usize,
    overlap_factor: usize,
    zero_pad_factor: usize,
    freq_ratio: i32,
) -> f32 {
    let cycle_length = (overlap_factor * zero_pad_factor) as isize;
    let cycle_idx = ((2 * (cycle_length) + hop_idx).max(0)) % cycle_length;

    let psr_minus_pad =
        (freq_ratio as isize + (cycle_length - zero_pad_factor as isize)) % cycle_length;

    // s * N
    let shift = (frame_size as isize * cycle_idx * psr_minus_pad / cycle_length) as usize;

    // (k*t+s*N) % N
    let offset = (frame_idx * (freq_ratio as usize) / zero_pad_factor + shift) % frame_size;

    original_analysis_window[offset]
}
