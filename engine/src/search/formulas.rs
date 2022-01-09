use crate::eval::Eval;

use super::window::Window;

pub const LMR_MIN_DEPTH: u8 = 3;

pub fn nmp_calculate_reduction(static_eval: Eval, window: Window) -> u8 {
    let mut reduction = 3;
    if let (Some(eval), Some(beta)) = (static_eval.as_cp(), window.beta.as_cp()) {
        reduction += ((eval - beta) / 100).min(2) as u8;
    }
    reduction
}

pub fn lmr_calculate_reduction(i: usize, depth: u8) -> u8 {
    if i < 2 {
        0
    } else if depth < 7 {
        1
    } else {
        2
    }
}

pub fn futility_margin(depth: u8) -> Option<Eval> {
    Some(Eval::cp(match depth {
        1 => 300,
        2 => 600,
        _ => return None
    }))
}

pub fn reverse_futility_margin(depth: u8) -> Option<Eval> {
    if depth < 5 {
        Some(Eval::cp(100 * depth as i16))
    } else {
        None
    }
}
