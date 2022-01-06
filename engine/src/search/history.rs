use cozy_chess::*;

pub struct HistoryTable([[[i32; Square::NUM]; Piece::NUM]; Color::NUM]);

impl HistoryTable {
    pub fn new() -> Self {
        Self([[[0; Square::NUM]; Piece::NUM]; Color::NUM])
    }

    pub fn get(&self, board: &Board, mv: Move) -> i32 {
        self.0
            [board.side_to_move() as usize]
            [board.piece_on(mv.from).unwrap() as usize]
            [mv.to as usize]
    }

    pub fn get_mut(&mut self, board: &Board, mv: Move) -> &mut i32 {
        &mut self.0
            [board.side_to_move() as usize]
            [board.piece_on(mv.from).unwrap() as usize]
            [mv.to as usize]
    }
}
