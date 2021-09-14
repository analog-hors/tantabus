use cozy_chess::*;

pub fn move_is_quiet(mv: Move, board: &Board) -> bool {
    !board.occupied().has(mv.to) && mv.promotion.is_none()
}
