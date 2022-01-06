use crate::eval::Eval;

pub const NULL_MOVE_REDUCTION: u8 = 2;
pub const LMR_MIN_DEPTH: u8 = 3;

pub fn lmr_calculate_reduction(i: usize, depth: u8) -> u8 {
    if i < 3 {
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

pub fn update_history(old: u32, depth: u8) -> u32 {
    let depth = depth as u32;
    old * 90 / 100 + (depth * depth)
}
