use cozy_chess::*;

pub struct ChessGame {
    board: Board,
    history: Vec<u64>,
    moves: Vec<Move>
}

impl ChessGame {
    pub fn new() -> Self {
        let board = Board::default();
        let history = vec![board.hash()];
        let moves = Vec::new();
        Self { board, history, moves }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    pub fn into_moves(self) -> Vec<Move> {
        self.moves
    }

    pub fn play_unchecked(&mut self, mv: Move) {
        self.board.play_unchecked(mv);
        self.moves.push(mv);
        self.history.push(self.board.hash());
    }

    pub fn game_status(&self) -> GameStatus {
        let status = self.board.status();
        if status != GameStatus::Ongoing {
            return status;
        }
        let repetitions = self.history.iter()
            .rev()
            .take(self.board.halfmove_clock() as usize + 1)
            .step_by(2) // Every second ply so it's our turn
            .filter(|&&hash| hash == self.board.hash())
            .count();
        if repetitions >= 3 {
            return GameStatus::Drawn;
        }
        GameStatus::Ongoing
    }
}
