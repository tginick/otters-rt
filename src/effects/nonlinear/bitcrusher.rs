use crate::conf::{AdvertisedParameter, BoardEffectConfigParameterValue, ParameterRange};
use crate::effects::basic_single_in_single_out;
use crate::traits::AudioEffect;

const PARAMS: &'static [AdvertisedParameter] = &[AdvertisedParameter {
    name: "quantized_bit_depth",
    range: ParameterRange::N(1, 15),
    default_value: BoardEffectConfigParameterValue::N(6),
}];

const PARAM_QUANTIZED_BIT_DEPTH: usize = 0;

pub struct BitCrusher {
    params: Vec<BoardEffectConfigParameterValue>,
    ql: f32,
}

impl BitCrusher {
    pub fn new() -> BitCrusher {
        let mut params = Vec::with_capacity(PARAMS.len());
        for i in 0..PARAMS.len() {
            params.push(PARAMS[i].default_value);
        }

        let default_ql =
            2.0f32 / (2.0f32.powf(params[PARAM_QUANTIZED_BIT_DEPTH].as_flt()) - 1.0f32);

        BitCrusher {
            params,
            ql: default_ql,
        }
    }

    pub fn info() -> &'static [AdvertisedParameter] {
        PARAMS
    }
}

impl AudioEffect for BitCrusher {
    fn advertise_parameters(&self) -> &'static [AdvertisedParameter] {
        BitCrusher::info()
    }

    fn set_audio_parameters(&mut self, _new_config: &crate::conf::AudioConfig) {}
    fn set_effect_parameter(
        &mut self,
        param_idx: usize,
        param_value: BoardEffectConfigParameterValue,
    ) {
        self.params[param_idx] = param_value;

        if param_idx == PARAM_QUANTIZED_BIT_DEPTH {
            self.ql =
                2.0f32 / (2.0f32.powf(self.params[PARAM_QUANTIZED_BIT_DEPTH].as_flt()) - 1.0f32);
        }
    }

    fn execute(
        &self,
        context: &crate::context::BoardContext,
        connection_idx: usize,
        num_samples: usize,
    ) {
        let maybe_bufs = basic_single_in_single_out(context, connection_idx, num_samples);
        if let None = maybe_bufs {
            return;
        }

        let (read_buf, mut write_buf) = maybe_bufs.unwrap();
        for i in 0..num_samples {
            let s = self.ql * (read_buf.buf_read(i) / self.ql).floor();
            write_buf.buf_write(i, s);
        }
    }
}
