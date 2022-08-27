use num_traits::PrimInt;

use super::ops::*;

#[derive(Debug, Clone)]
pub struct Linear<W, B, const INPUTS: usize, const OUTPUTS: usize> {
    pub weights: [[W; INPUTS]; OUTPUTS],
    pub biases: [B; OUTPUTS]
}

impl<
    W: PrimInt, B: PrimInt,
    const INPUTS: usize,
    const OUTPUTS: usize
> Linear<W, B, INPUTS, OUTPUTS> where [W; INPUTS]: Dot<B> {
    pub fn activate(&self, inputs: &[W; INPUTS], outputs: &mut [B; OUTPUTS]) {
        *outputs = self.biases;
        for (o, w) in outputs.iter_mut().zip(&self.weights) {
            *o = o.add(inputs.dot(w));
        }
    }
}

#[derive(Debug, Clone)]
pub struct BitLinear<WB, const INPUTS: usize, const OUTPUTS: usize> {
    pub weights: [[WB; OUTPUTS]; INPUTS],
    pub biases: [WB; OUTPUTS]
}

impl<
    WB: PrimInt,
    const INPUTS: usize,
    const OUTPUTS: usize
> BitLinear<WB, INPUTS, OUTPUTS>
where
    [WB; OUTPUTS]: VecAdd + VecSub {
    pub fn empty(&self, outputs: &mut [WB; OUTPUTS]) {
        *outputs = self.biases;
    }

    pub fn add(&self, index: usize, outputs: &mut [WB; OUTPUTS]) {
        outputs.vec_add(&self.weights[index]);
    }

    pub fn sub(&self, index: usize, outputs: &mut [WB; OUTPUTS]) {
        outputs.vec_sub(&self.weights[index]);
    }
}
