pub type LinearW = i8;
pub type LinearB = i32;

#[derive(Debug, Clone)]
pub struct Linear<const INPUTS: usize, const OUTPUTS: usize> {
    pub weights: [[LinearW; INPUTS]; OUTPUTS],
    pub biases: [LinearB; OUTPUTS]
}

impl<const INPUTS: usize, const OUTPUTS: usize> Linear<INPUTS, OUTPUTS> {
    pub fn activate(&self, inputs: &[LinearW; INPUTS], outputs: &mut [LinearB; OUTPUTS]) {
        *outputs = self.biases;
        for (o, w) in outputs.iter_mut().zip(&self.weights) {
            *o += dot_product(inputs, w);
        }
    }
}

pub type BitLinearWB = i16;

#[derive(Debug, Clone)]
pub struct BitLinear<const INPUTS: usize, const OUTPUTS: usize> {
    pub weights: [[BitLinearWB; OUTPUTS]; INPUTS],
    pub biases: [BitLinearWB; OUTPUTS]
}

impl<const INPUTS: usize, const OUTPUTS: usize> BitLinear<INPUTS, OUTPUTS> {
    pub fn empty(&self, outputs: &mut [BitLinearWB; OUTPUTS]) {
        *outputs = self.biases;
    }

    pub fn add(&self, index: usize, outputs: &mut [BitLinearWB; OUTPUTS]) {
        vec_add(outputs, &self.weights[index]);
    }

    pub fn sub(&self, index: usize, outputs: &mut [BitLinearWB; OUTPUTS]) {
        vec_sub(outputs, &self.weights[index]);
    }
}

fn dot_product<const LEN: usize>(vec: &[LinearW; LEN], other: &[LinearW; LEN]) -> LinearB {
    vec.iter().zip(other).map(|(&v, &o)| v as LinearB * o as LinearB).sum()
}

fn vec_add<const LEN: usize>(vec: &mut [BitLinearWB; LEN], other: &[BitLinearWB; LEN]) {
    for (v, o) in vec.iter_mut().zip(other) {
        *v += o;
    }
}

fn vec_sub<const LEN: usize>(vec: &mut [BitLinearWB; LEN], other: &[BitLinearWB; LEN]) {
    for (v, o) in vec.iter_mut().zip(other) {
        *v -= o;
    }
}
