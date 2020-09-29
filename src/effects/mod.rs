mod biquad_filter;
mod bypass;
mod delay;
mod dynamics;
mod misc_vocoder;
mod modulation;
mod nonlinear;
mod pitch;
mod reverb;
mod vocoder2;

use crate::conf::{AdvertisedParameter, AudioConfig};
use crate::context::BoardContext;
use crate::traits::AudioEffect;
use crate::utils::buf_rw::{AudioBufferReader, AudioBufferWriter};

use fftw::array::AlignedVec;

use std::collections::HashMap;

pub use bypass::GenericBypass;

pub type AudioEffectConstructionFunction = Box<dyn Fn(AudioConfig) -> Box<dyn AudioEffect>>;
pub type AudioEffectInformationFunction = Box<dyn Fn() -> &'static [AdvertisedParameter]>;

pub struct AudioEffectConstructionInfo {
    pub constructor: AudioEffectConstructionFunction,
    pub info: AudioEffectInformationFunction,
}

pub struct VocoderContext {
    pub hop_size: usize,
    pub frame_size: usize,
    pub analysis_window: AlignedVec<f32>,
}

pub struct FactoryExtension {
    pub factory_fns: HashMap<&'static str, AudioEffectConstructionInfo>,
}

fn bypass_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "Bypass/Mono",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| Box::new(bypass::MonoBypass::new())),
            info: Box::new(|| bypass::MonoBypass::info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn delay_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "Delay/Basic",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(delay::MonoDelayBasic::new(ac))),
            info: Box::new(|| delay::MonoDelayBasic::info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn modulation_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "Modulation/Phaser",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(modulation::MonoPhaser::new(ac))),
            info: Box::new(|| modulation::MonoPhaser::info()),
        },
    );

    factory_fns.insert(
        "Modulation/Flanger",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(modulation::ModulatedDelay::new_flanger(ac))),
            info: Box::new(|| modulation::ModulatedDelay::modulated_delay_info()),
        },
    );

    factory_fns.insert(
        "Modulation/Chorus",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(modulation::ModulatedDelay::new_chorus(ac))),
            info: Box::new(|| modulation::ModulatedDelay::modulated_delay_info()),
        },
    );

    factory_fns.insert(
        "Modulation/Vibrato",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(modulation::ModulatedDelay::new_vibrato(ac))),
            info: Box::new(|| modulation::ModulatedDelay::modulated_delay_info()),
        },
    );

    factory_fns.insert(
        "Modulation/WhiteChorus",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(modulation::ModulatedDelay::new_white_chorus(ac))),
            info: Box::new(|| modulation::ModulatedDelay::modulated_delay_info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn nonlinear_processing_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "NonLinear/BitCrusher",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| Box::new(nonlinear::BitCrusher::new())),
            info: Box::new(|| nonlinear::BitCrusher::info()),
        },
    );

    factory_fns.insert(
        "NonLinear/WaveShaper",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| Box::new(nonlinear::WaveShaper::new())),
            info: Box::new(|| nonlinear::WaveShaper::info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn misc_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "Filter/Biquad",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(biquad_filter::BiquadFilter::new(ac))),
            info: Box::new(|| biquad_filter::BiquadFilter::info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn dynamics_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "Dynamics/BasicCompressor",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(dynamics::Dynamics::new_compressor(ac))),
            info: Box::new(|| dynamics::Dynamics::dynamics_info()),
        },
    );

    factory_fns.insert(
        "Dynamics/BasicDownwardExpander",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(dynamics::Dynamics::new_expander(ac))),
            info: Box::new(|| dynamics::Dynamics::dynamics_info()),
        },
    );

    factory_fns.insert(
        "Dynamics/BasicLimiter",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(dynamics::Dynamics::new_limiter(ac))),
            info: Box::new(|| dynamics::Dynamics::dynamics_info()),
        },
    );

    factory_fns.insert(
        "Dynamics/BasicNoiseGate",
        AudioEffectConstructionInfo {
            constructor: Box::new(|ac| Box::new(dynamics::Dynamics::new_gate(ac))),
            info: Box::new(|| dynamics::Dynamics::dynamics_info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn vocoder_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    factory_fns.insert(
        "PitchShifter/Ocean",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| {
                Box::new(vocoder2::PhaseVocoder::new(
                    1024,
                    256,
                    vocoder2::FFTWindowType::Hann,
                    pitch::OceanPitchShifter::new(),
                ))
            }),
            info: Box::new(|| pitch::OceanPitchShifter::info()),
        },
    );

    factory_fns.insert(
        "Vocoder/Bypass",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| {
                Box::new(vocoder2::PhaseVocoder::new(
                    1024,
                    256,
                    vocoder2::FFTWindowType::Hamming,
                    bypass::VocoderBypass::new(),
                ))
            }),
            info: Box::new(|| bypass::VocoderBypass::info()),
        },
    );

    factory_fns.insert(
        "Vocoder/Robotize",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| {
                Box::new(vocoder2::PhaseVocoder::new(
                    1024,
                    256,
                    vocoder2::FFTWindowType::Hamming,
                    misc_vocoder::Robotize::new(),
                ))
            }),
            info: Box::new(|| misc_vocoder::Robotize::info()),
        },
    );

    factory_fns.insert(
        "Vocoder/Whisper",
        AudioEffectConstructionInfo {
            constructor: Box::new(|_ac| {
                Box::new(vocoder2::PhaseVocoder::new(
                    1024,
                    256,
                    vocoder2::FFTWindowType::Hamming,
                    misc_vocoder::Whisper::new(),
                ))
            }),
            info: Box::new(|| misc_vocoder::Whisper::info()),
        },
    );

    FactoryExtension { factory_fns }
}

fn reverb_effects() -> FactoryExtension {
    let mut factory_fns: HashMap<&'static str, AudioEffectConstructionInfo> = HashMap::new();

    FactoryExtension { factory_fns }
}

// configure which effect sets are loaded if desired
pub fn loaded_set() -> Vec<FactoryExtension> {
    return vec![
        bypass_effects(),
        delay_effects(),
        misc_effects(),
        modulation_effects(),
        nonlinear_processing_effects(),
        dynamics_effects(),
        vocoder_effects(),
        reverb_effects(),
    ];
}

pub fn basic_single_in_single_out(
    context: &BoardContext,
    connection_idx: usize,
    num_samples: usize,
) -> Option<(AudioBufferReader, AudioBufferWriter)> {
    let inputs = context.get_inputs_for_connection(connection_idx);
    let outputs = context.get_outputs_for_connection(connection_idx);

    if outputs.len() < 1 {
        return None;
    }

    let mut write_buf = context.get_buffer_for_write(outputs[0]);

    if inputs.len() < 1 {
        for i in 0..num_samples {
            write_buf.buf_write(i, 0.0f32);
        }

        return None;
    }

    let read_buf = context.get_buffer_for_read(inputs[0]);
    Some((read_buf, write_buf))
}
