use super::mathutils::{db_to_linear, vcosf, vsinf, vtanf};
use super::ringbuf::TinyFloatBuffer;

use num_derive::FromPrimitive;
use std::mem;

const DEFAULT_Q: f32 = 0.707f32;

#[derive(Clone, FromPrimitive)]
#[allow(non_camel_case_types)]
pub enum IIRFilterType {
    FirstOrderLowPass = 0,
    SecondOrderLowPass,
    FirstOrderHighPass,
    SecondOrderHighPass,
    SecondOrderBandPass,
    SecondOrderBandStop,
    FirstOrderAllPass,
    SecondOrderAllPass,
    FirstOrderLowShelf,
    FirstOrderHighShelf,

    __NUM_IIR_FILTER_TYPES,
}

#[derive(Clone)]
pub struct BiquadCoefficients {
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    c0: f32,
    d0: f32,
    cutoff: f32,
    q: f32,
    sample_rate: f32,
    shelf_gain_db: f32,
    iir_type: IIRFilterType,
}

pub struct Biquad {
    coefficients: BiquadCoefficients,
    x: TinyFloatBuffer,
    y: TinyFloatBuffer,
}

impl Default for IIRFilterType {
    fn default() -> Self {
        IIRFilterType::FirstOrderLowPass
    }
}

impl BiquadCoefficients {
    pub fn set_cutoff(mut self, new_cutoff: f32) -> BiquadCoefficients {
        self.cutoff = new_cutoff;
        self.recreate()
    }

    pub fn set_sample_rate(mut self, new_sample_rate: f32) -> BiquadCoefficients {
        self.sample_rate = new_sample_rate;
        self.recreate()
    }

    pub fn set_q(mut self, new_q: f32) -> BiquadCoefficients {
        self.q = new_q;
        self.recreate()
    }

    pub fn set_shelf_gain_db(mut self, new_shelf_gain_db: f32) -> BiquadCoefficients {
        self.shelf_gain_db = new_shelf_gain_db;
        self.recreate()
    }

    pub fn change_type(mut self, new_type: IIRFilterType) -> BiquadCoefficients {
        self.iir_type = new_type;
        self.recreate()
    }

    fn recreate(&self) -> BiquadCoefficients {
        match &self.iir_type {
            IIRFilterType::FirstOrderLowPass => {
                BiquadCoefficients::first_order_lpf(self.cutoff, self.sample_rate)
            }
            IIRFilterType::SecondOrderLowPass => {
                BiquadCoefficients::second_order_lpf(self.cutoff, self.sample_rate, Some(self.q))
            }
            IIRFilterType::FirstOrderHighPass => {
                BiquadCoefficients::first_order_hpf(self.cutoff, self.sample_rate)
            }
            IIRFilterType::SecondOrderHighPass => {
                BiquadCoefficients::second_order_hpf(self.cutoff, self.sample_rate, Some(self.q))
            }
            IIRFilterType::FirstOrderAllPass => {
                BiquadCoefficients::first_order_apf(self.cutoff, self.sample_rate)
            }
            IIRFilterType::SecondOrderAllPass => {
                BiquadCoefficients::second_order_apf(self.cutoff, self.sample_rate, Some(self.q))
            }
            IIRFilterType::SecondOrderBandPass => {
                BiquadCoefficients::second_order_bpf(self.cutoff, self.sample_rate, Some(self.q))
            }
            IIRFilterType::SecondOrderBandStop => {
                BiquadCoefficients::second_order_bsf(self.cutoff, self.sample_rate, Some(self.q))
            }
            IIRFilterType::FirstOrderLowShelf => {
                BiquadCoefficients::first_order_low_shelf(self.cutoff, self.sample_rate, self.shelf_gain_db)
            }
            IIRFilterType::FirstOrderHighShelf => {
                BiquadCoefficients::first_order_high_shelf(self.cutoff, self.sample_rate, self.shelf_gain_db)
            }

            IIRFilterType::__NUM_IIR_FILTER_TYPES => panic!("Should never get here"),
        }
    }

    pub fn first_order_lpf(cutoff: f32, sample_rate: f32) -> BiquadCoefficients {
        let theta_c = super::TWO_PI * cutoff / sample_rate;
        let gamma = vcosf(theta_c) / (1.0f32 + vsinf(theta_c));

        let a0 = (1.0f32 - gamma) / 2.0f32;
        let a1 = a0;
        let a2 = 0.0f32;
        let b1 = -gamma;
        let b2 = 0.0f32;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff,
            sample_rate,
            q: DEFAULT_Q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::FirstOrderLowPass,
        }
    }

    pub fn second_order_lpf(cutoff: f32, sample_rate: f32, q: Option<f32>) -> BiquadCoefficients {
        let q = q.unwrap_or(DEFAULT_Q);

        let theta_c = super::TWO_PI * cutoff / sample_rate;
        let d2 = 1.0f32 / q / 2.0f32;
        let sinf_theta_c = vsinf(theta_c);
        let beta = 0.5f32 * (1.0f32 - d2 * sinf_theta_c) / (1.0f32 + d2 * sinf_theta_c);
        let gamma = (0.5f32 + beta) * vcosf(theta_c);

        let a0 = (0.5f32 + beta - gamma) / 2.0f32;
        let a1 = 2.0f32 * a0;
        let a2 = a0;
        let b1 = -2.0f32 * gamma;
        let b2 = 2.0f32 * beta;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff,
            sample_rate,
            q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::SecondOrderLowPass,
        }
    }

    pub fn first_order_hpf(cutoff: f32, sample_rate: f32) -> BiquadCoefficients {
        let theta_c = super::TWO_PI * cutoff / sample_rate;
        let gamma = vcosf(theta_c) / (1.0f32 + vsinf(theta_c));

        let a0 = (1.0f32 + gamma) / 2.0f32;
        let a1 = -a0;
        let a2 = 0.0f32;
        let b1 = -gamma;
        let b2 = 0.0f32;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff,
            sample_rate,
            q: DEFAULT_Q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::FirstOrderHighPass,
        }
    }

    pub fn second_order_hpf(cutoff: f32, sample_rate: f32, q: Option<f32>) -> BiquadCoefficients {
        let q = q.unwrap_or(DEFAULT_Q);

        let theta_c = super::TWO_PI * cutoff / sample_rate;
        let d2 = 1.0f32 / q / 2.0f32;
        let sinf_theta_c = vsinf(theta_c);
        let beta = 0.5f32 * (1.0f32 - d2 * sinf_theta_c) / (1.0f32 + d2 * sinf_theta_c);
        let gamma = (0.5f32 + beta) * vcosf(theta_c);

        let a0 = (0.5f32 + beta + gamma) / 2.0f32;
        let a1 = -2.0f32 * a0;
        let a2 = a0;
        let b1 = -2.0f32 * gamma;
        let b2 = 2.0f32 * beta;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff,
            sample_rate,
            q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::SecondOrderHighPass,
        }
    }

    pub fn second_order_bpf(corner: f32, sample_rate: f32, q: Option<f32>) -> BiquadCoefficients {
        let q = q.unwrap_or(DEFAULT_Q);
        let k = vtanf(std::f32::consts::PI * corner / sample_rate);
        let delta = k * k * q + k + q;

        let a0 = k / delta;
        let a1 = 0.0f32;
        let a2 = -k / delta;
        let b1 = (2.0f32 * q * (k * k - 1.0f32)) / delta;
        let b2 = (k * k * q - k + q) / delta;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff: corner,
            sample_rate,
            q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::SecondOrderBandPass,
        }
    }

    pub fn second_order_bsf(corner: f32, sample_rate: f32, q: Option<f32>) -> BiquadCoefficients {
        let q = q.unwrap_or(DEFAULT_Q);
        let k = vtanf(std::f32::consts::PI * corner / sample_rate);
        let delta = k * k * q + k + q;

        let a0 = (q * (k * k + 1.0f32)) / delta;
        let a1 = (2.0f32 * q * (k * k - 1.0f32)) / delta;
        let a2 = a0;
        let b1 = a1;
        let b2 = (k * k * q - k + q) / delta;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff: corner,
            sample_rate,
            q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::SecondOrderBandStop,
        }
    }

    pub fn first_order_apf(corner: f32, sample_rate: f32) -> BiquadCoefficients {
        // TODO: corner MUST be less than 0.5* sample_rate (nyquist freq)
        // tan is undefined
        let theta_c = std::f32::consts::PI * corner / sample_rate;
        let tan_theta_c = vtanf(theta_c);
        let alpha = (tan_theta_c - 1.0f32) / (tan_theta_c + 1.0f32);

        let a0 = alpha;
        let a1 = 1.0f32;
        let a2 = 0.0f32;
        let b1 = alpha;
        let b2 = 0.0f32;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff: corner,
            sample_rate,
            q: DEFAULT_Q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::FirstOrderAllPass,
        }
    }

    pub fn second_order_apf(corner: f32, sample_rate: f32, q: Option<f32>) -> BiquadCoefficients {
        let q = q.unwrap_or(DEFAULT_Q);
        let w = corner * std::f32::consts::PI / q / sample_rate;
        // TODO: w must be < PI / 2

        let tan_w = vtanf(w);

        let alpha = (tan_w - 1.0f32) / (tan_w + 1.0f32);
        let beta = -vcosf(super::TWO_PI * corner / sample_rate);

        let a0 = -alpha;
        let a1 = beta * (1.0f32 - alpha);
        let a2 = 1.0f32;
        let b1 = a1;
        let b2 = a0;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0: 1.0f32,
            d0: 0.0f32,
            cutoff: corner,
            sample_rate,
            q,
            shelf_gain_db: 0.0f32,
            iir_type: IIRFilterType::SecondOrderAllPass,
        }
    }

    pub fn first_order_low_shelf(shelf_freq: f32, sample_rate: f32, gain_db: f32) -> BiquadCoefficients {
        let theta_c = super::TWO_PI * shelf_freq / sample_rate;
        let mu = db_to_linear(gain_db);
        let beta = 4.0f32 / ( 1.0f32 + mu);
        let delta = beta * vtanf(theta_c / 2.0f32);
        let gamma = (1.0f32 - delta) / (1.0f32 + delta);
        
        let a0 = (1.0f32 - gamma) / 2.0f32;
        let a1 = a0;
        let a2 = 0.0f32;
        let b1 = -gamma;
        let b2 = 0.0f32;
        let c0 = mu - 1.0f32;
        let d0 = 1.0f32;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0,
            d0,
            cutoff:shelf_freq,
            sample_rate,
            q: DEFAULT_Q,
            shelf_gain_db: gain_db,
            iir_type: IIRFilterType::FirstOrderLowShelf,
        }
    }

    pub fn first_order_high_shelf(shelf_freq: f32, sample_rate: f32, gain_db: f32) -> BiquadCoefficients {
        let theta_c = super::TWO_PI * shelf_freq / sample_rate;
        let mu = db_to_linear(gain_db);
        let beta = (1.0f32 + mu) / 4.0f32;
        let delta = beta * vtanf(theta_c / 2.0f32);
        let gamma = (1.0f32 - delta) / (1.0f32 + delta);
        let a0 = (1.0f32 + gamma) / 2.0f32;
        let a1 = -a0;
        let a2 = 0.0f32;
        let b1 = -gamma;
        let b2 = 0.0f32;
        let c0 = mu - 1.0f32;
        let d0 = 1.0f32;

        BiquadCoefficients {
            a0,
            a1,
            a2,
            b1,
            b2,
            c0,
            d0,
            cutoff:shelf_freq,
            sample_rate,
            q: DEFAULT_Q,
            shelf_gain_db: gain_db,
            iir_type: IIRFilterType::FirstOrderHighShelf,
        }
    }
}

impl Biquad {
    pub fn new(coeff: BiquadCoefficients) -> Biquad {
        Biquad {
            coefficients: coeff,
            x: TinyFloatBuffer::new(),
            y: TinyFloatBuffer::new(),
        }
    }

    pub fn change_sample_rate(&mut self, new_sample_rate: f32) {
        let mut temp = self.coefficients.clone();

        mem::swap(&mut self.coefficients, &mut temp);
        temp = temp.set_sample_rate(new_sample_rate);
        mem::swap(&mut self.coefficients, &mut temp);
    }

    pub fn change_type(&mut self, new_type: IIRFilterType) {
        let mut temp = self.coefficients.clone();

        mem::swap(&mut self.coefficients, &mut temp);
        temp = temp.change_type(new_type);
        mem::swap(&mut self.coefficients, &mut temp);
    }

    pub fn change_cutoff(&mut self, new_cutoff: f32) {
        let mut temp = self.coefficients.clone();

        mem::swap(&mut self.coefficients, &mut temp);
        temp = temp.set_cutoff(new_cutoff);
        mem::swap(&mut self.coefficients, &mut temp);
    }

    pub fn change_shelf_gain(&mut self, new_gain: f32) {
        let mut temp = self.coefficients.clone();
        
        mem::swap(&mut self.coefficients, &mut temp);
        temp = temp.set_shelf_gain_db(new_gain);
        mem::swap(&mut self.coefficients, &mut temp);
    }

    pub fn change_q(&mut self, new_q: f32) {
        let mut temp = self.coefficients.clone();
        
        mem::swap(&mut self.coefficients, &mut temp);
        temp = temp.set_q(new_q);
        mem::swap(&mut self.coefficients, &mut temp);
    }

    pub fn change_params(&mut self, new_params: BiquadCoefficients) {
        self.coefficients = new_params;
    }

    pub fn filter(&mut self, input: f32) -> f32 {
        // y(n) = c_0 * (a_0 * x(n) + a_1 * x(n - 1) + a_2 * x(n - 2) - b_1 * y(n - 1) - b_2 * y (n - 2)) + d_0 * x(n)
        // TODO: low hanging fruit for vectorization
        let result = self.coefficients.c0
            * (self.coefficients.a0 * input
                + self.coefficients.a1 * self.x.z1()
                + self.coefficients.a2 * self.x.z2()
                - self.coefficients.b1 * self.y.z1()
                - self.coefficients.b2 * self.y.z2())
            + self.coefficients.d0 * input;

        self.x.write(input);
        self.y.write(result);

        result
    }

    pub fn g(&self) -> f32 {
        self.coefficients.a0
    }

    pub fn s(&self) -> f32 {
        self.coefficients.a1 * self.x.z1() + self.coefficients.a2 * self.x.z2()
            - self.coefficients.b1 * self.y.z1()
            - self.coefficients.b2 * self.y.z2()
    }
}
