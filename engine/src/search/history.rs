use cozy_chess::*;

pub struct HistoryTable([[[i32; Square::NUM]; Piece::NUM]; Color::NUM]);

const MAX_HISTORY: i32 = 2048;

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

    pub fn update(&mut self, board: &Board, mv: Move, depth: u8, cutoff: bool) {
        let history = &mut self.0
            [board.side_to_move() as usize]
            [board.piece_on(mv.from).unwrap() as usize]
            [mv.to as usize];
        let change = depth as i32 * depth as i32;
        let decay = change * *history / MAX_HISTORY;
        if cutoff {
            *history += change;
        } else {
            *history -= change;
        }
        *history -= decay;
        *history = (*history).clamp(-MAX_HISTORY, MAX_HISTORY);
    }
}
