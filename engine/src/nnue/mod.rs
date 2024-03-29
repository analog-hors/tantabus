use cozy_chess::*;

mod layers;

use self::layers::*;

const FEATURES: usize = 768;
const FT_OUT: usize = 256;
const L1_OUT: usize = 1;

const ACTIVATION_RANGE: i8 = 127;
const WEIGHT_SCALE: i8 = 64;
const OUTPUT_SCALE: i32 = 115;

#[derive(Debug, Clone)]
pub struct Nnue {
    pub ft: BitLinear<FEATURES, FT_OUT>,
    pub l1: Linear<{FT_OUT * Color::NUM}, L1_OUT>
}

impl Nnue {
    pub const DEFAULT: Self = include!("model.txt");

    pub fn new_state(&self) -> NnueState<'_> {
        let mut accumulator = [[0; FT_OUT]; Color::NUM];
        self.ft.empty(&mut accumulator[Color::White as usize]);
        self.ft.empty(&mut accumulator[Color::Black as usize]);
        NnueState {
            model: self,
            accumulator
        }
    }
}

#[derive(Debug, Clone)]
pub struct NnueState<'m> {
    model: &'m Nnue,
    accumulator: [[i16; FT_OUT]; Color::NUM]
}

pub fn feature(perspective: Color, color: Color, piece: Piece, square: Square) -> usize {
    let (square, color) = match perspective {
        Color::White => (square, color),
        Color::Black => (square.flip_rank(), !color),
    };
    let mut index = 0;
    index = index * Color::NUM + color as usize;
    index = index * Piece::NUM + piece as usize;
    index = index * Square::NUM + square as usize;
    index
}

impl<'s> NnueState<'s> {
    pub fn model(&self) -> &Nnue {
        self.model
    }

    pub fn accumulator(&self) -> &[[i16; FT_OUT]; Color::NUM] {
        &self.accumulator
    }

    pub fn add(&mut self, color: Color, piece: Piece, square: Square) {
        for &perspective in &Color::ALL {
            let feature = feature(perspective, color, piece, square);
            self.model.ft.add(feature, &mut self.accumulator[perspective as usize]);
        }
    }

    pub fn sub(&mut self, color: Color, piece: Piece, square: Square) {
        for &perspective in &Color::ALL {
            let feature = feature(perspective, color, piece, square);
            self.model.ft.sub(feature, &mut self.accumulator[perspective as usize]);
        }
    }

    pub fn evaluate(&self, side_to_move: Color) -> i32 {
        let mut inputs = [[0; FT_OUT]; Color::NUM];
        clipped_relu(&self.accumulator[side_to_move as usize], &mut inputs[0]);
        clipped_relu(&self.accumulator[!side_to_move as usize], &mut inputs[1]);
        let inputs = bytemuck::cast(inputs);
        let mut outputs = [0; L1_OUT];
        self.model.l1.activate(&inputs, &mut outputs);
        outputs[0] * OUTPUT_SCALE / WEIGHT_SCALE as LinearB / ACTIVATION_RANGE as LinearB
    }
}

fn clipped_relu<const LEN: usize>(vec: &[BitLinearWB; LEN], out: &mut [LinearI; LEN]) {
    for (&v, o) in vec.iter().zip(out) {
        *o = v.clamp(0, ACTIVATION_RANGE as BitLinearWB) as LinearI;
    }
}
