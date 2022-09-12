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
        bonus_reduction_index: usize = 3;
        bonus_reduction_min_depth: u8 = 7;
        history_reduction_div: i32 = 205;
    }
    nmp = NmpParams {
        base_reduction: u8 = 3;
        margin_div: i32 = 90;
        margin_max_reduction: u8 = 2;
    }
    lmp = LmpParams {
        quiets_to_check: [usize; 4] = [7, 8, 17, 20];
    }
    fp = FpParams {
        margins: [i16; 2] = [293, 622];
    }
    rfp = RfpParams {
        base_margin: i16 = 33;
        max_depth: u8 = 4;
    }
}

impl LmrParams {
    pub fn reduction(&self, i: usize, depth: u8, history: i32) -> u8 {
        let mut reduction: i8 = if i < self.bonus_reduction_index {
            0
        } else if depth < self.bonus_reduction_min_depth {
            1
        } else {
            2
        };
        reduction -= (history / self.history_reduction_div) as i8;
        reduction.max(0) as u8
    }
}

impl NmpParams {
    pub fn reduction(&self, static_eval: Eval, window: Window) -> u8 {
        let mut reduction = self.base_reduction;
        if let (Some(eval), Some(beta)) = (static_eval.as_cp(), window.beta.as_cp()) {
            if eval >= beta {
                // CITE: This kind of reduction increase when eval >= beta was first observed in MadChess.
                // https://www.madchess.net/2021/02/09/madchess-3-0-beta-f231dac-pvs-and-null-move-improvements/
                reduction += ((eval as i32 - beta as i32) / self.margin_div)
                    .min(self.margin_max_reduction as i32) as u8;
            }
        }
        reduction
    }
}

impl LmpParams {
    pub fn quiets_to_check(&self, depth: u8) -> usize {
        *self.quiets_to_check.get(depth as usize - 1)
            .unwrap_or(&usize::MAX)
    }
}

impl FpParams {
    pub fn margin(&self, depth: u8) -> Option<Eval> {
        self.margins.get(depth as usize - 1)
            .map(|&e| Eval::cp(e))
    }
}

impl RfpParams {
    pub fn margin(&self, depth: u8) -> Option<Eval> {
        if depth <= self.max_depth {
            Some(Eval::cp(self.base_margin * depth as i16))
        } else {
            None
        }
    }
}
