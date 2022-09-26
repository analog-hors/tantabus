use crate::eval::Eval;

use super::window::Window;

macro_rules! define_params {
    ($($name:ident = $params_name:ident {
        $($param:ident: $type:ty = $value:expr;)*
    })*) => {
        #[derive(Debug, Default, Clone)]
        pub struct SearchParams {
            $(pub $name: $params_name),*
        }

        $(#[derive(Debug, Clone)]
        pub struct $params_name {
            $(pub $param: $type),*
        }
    
        impl Default for $params_name {
            fn default() -> Self {
                Self {
                    $($param: $value),*
                }
            }
        })*
    }
}

define_params! {
    lmr = LmrParams {
        min_depth: u8 = 3;
        base_reduction: f32 = 0.007;
        div: f32 = 2.792;
        history_reduction_div: i32 = 210;
    }
    nmp = NmpParams {
        base_reduction: u8 = 3;
        bonus_reduction: u8 = 1;
        bonus_reduction_depth: u8 = 7;
        margin_div: i32 = 90;
        margin_max_reduction: u8 = 2;
    }
    lmp = LmpParams {
        quiets_to_check: [usize; 3] = [7, 8, 17];
    }
    fp = FpParams {
        margins: [i16; 2] = [293, 620];
    }
    rfp = RfpParams {
        base_margin: i16 = 30;
        max_depth: u8 = 4;
    }
}

struct Lut2d<T, const I: usize, const J: usize> {
    lut: [[T; J]; I]
}

impl<T: Copy + Default, const I: usize, const J: usize> Lut2d<T, I, J> {
    pub fn new(mut init: impl FnMut(usize, usize) -> T) -> Self {
        let mut lut = [[T::default(); J]; I];
        for i in 0..I {
            for j in 0..J {
                lut[i][j] = init(i, j);
            }
        }
        Self { lut }
    }

    pub fn get(&self, i: usize, j: usize) -> T {
        self.lut[i.min(I - 1)][j.min(J - 1)]
    }
}

pub struct SearchParamHandler {
    params: SearchParams,
    lmr_lut: Lut2d<u8, 64, 64>,
}

impl SearchParamHandler {
    pub fn new(params: SearchParams) -> Self {
        let lmr_lut = Lut2d::new(|depth, move_index| {
            let base = params.lmr.base_reduction;
            let div = params.lmr.div;
            (base + (depth as f32).ln() * (move_index as f32).ln() / div) as u8
        });
        Self { params, lmr_lut }
    }

    pub fn lmr_min_depth(&self) -> u8 {
        self.params.lmr.min_depth
    }

    pub fn lmr_reduction(&self, move_index: usize, depth: u8, history: i32) -> u8 {
        let mut reduction = self.lmr_lut.get(depth as usize, move_index) as i32;
        reduction -= history / self.params.lmr.history_reduction_div;
        reduction.max(0) as u8
    }

    pub fn nmp_reduction(&self, depth: u8, static_eval: Eval, window: Window) -> u8 {
        let nmp = &self.params.nmp;
        let mut reduction = nmp.base_reduction;
        if depth >= nmp.bonus_reduction_depth {
            reduction += nmp.bonus_reduction;
        }
        if let (Some(eval), Some(beta)) = (static_eval.as_cp(), window.beta.as_cp()) {
            if eval >= beta {
                // CITE: This kind of reduction increase when eval >= beta was first observed in MadChess.
                // https://www.madchess.net/2021/02/09/madchess-3-0-beta-f231dac-pvs-and-null-move-improvements/
                reduction += ((eval as i32 - beta as i32) / nmp.margin_div)
                    .min(nmp.margin_max_reduction as i32) as u8;
            }
        }
        reduction
    }

    pub fn lmp_quiets_to_check(&self, depth: u8) -> usize {
        *self.params.lmp.quiets_to_check.get(depth as usize - 1)
            .unwrap_or(&usize::MAX)
    }

    pub fn fp_margin(&self, depth: u8) -> Option<Eval> {
        self.params.fp.margins.get(depth as usize - 1)
            .map(|&e| Eval::cp(e))
    }

    pub fn rfp_margin(&self, depth: u8) -> Option<Eval> {
        let rfp = &self.params.rfp;
        if depth <= rfp.max_depth {
            Some(Eval::cp(rfp.base_margin * depth as i16))
        } else {
            None
        }
    }
}
