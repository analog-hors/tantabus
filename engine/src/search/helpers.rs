use cozy_chess::*;

pub fn move_is_capture(mv: Move, board: &Board) -> bool {
    board.colors(!board.side_to_move()).has(mv.to)
}

pub fn move_is_quiet(mv: Move, board: &Board) -> bool {
    !move_is_capture(mv, board) && mv.promotion.is_none()
}
