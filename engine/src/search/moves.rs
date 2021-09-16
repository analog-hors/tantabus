use cozy_chess::*;
use arrayvec::ArrayVec;

use crate::eval::{EVALUATOR, Eval};

use super::SearchHandler;
use super::search::{KillerEntry, Searcher};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveScore {
    LosingCapture(Eval),
    Quiet(u32),
    Killer,
    Capture(Eval),
    Pv
}

pub struct MoveList {
    move_list: ArrayVec<(Move, MoveScore), 218>
}

impl<H: SearchHandler> Searcher<'_, H> {
    pub fn new_movelist(&mut self, board: &Board, pv_move: Option<Move>, killers: KillerEntry) ->  MoveList {
        let mut move_list = ArrayVec::new();

        let their_pieces = board.colors(!board.side_to_move());
        board.generate_moves(|mut moves| {
            if let Some(pv_move) = pv_move {
                if moves.from == pv_move.from && moves.to.has(pv_move.to) {
                    moves.to ^= pv_move.to.bitboard();
                    move_list.push((pv_move, MoveScore::Pv));
                }
            }
            let mut capture_moves = moves;
            capture_moves.to &= their_pieces;
            let mut quiet_moves = moves;
            quiet_moves.to ^= capture_moves.to;

            for mv in quiet_moves {
                let score = if killers.contains(&mv) {
                    MoveScore::Killer
                } else {
                    let history = self.history_table
                        [board.side_to_move() as usize]
                        [board.piece_on(mv.from).unwrap() as usize]
                        [mv.to as usize];
                    MoveScore::Quiet(history)
                };
                move_list.push((mv, score));
            }

            for &victim in &Piece::ALL {
                let mut capture_moves = capture_moves;
                capture_moves.to &= board.pieces(victim);
                for mv in capture_moves {
                    let eval = EVALUATOR.piece_value(victim) * Eval::cp(1000)
                        - EVALUATOR.piece_value(moves.piece);
                    let score = if eval >= Eval::ZERO {
                        MoveScore::Capture(eval)
                    } else {
                        MoveScore::LosingCapture(eval)
                    };
                    move_list.push((mv, score));
                }
            }
            false
        });
        MoveList {
            move_list
        }
    }

    pub fn quiet_movelist(&mut self, board: &Board) ->  MoveList {
        let mut move_list = ArrayVec::new();

        let their_pieces = board.colors(!board.side_to_move());
        board.generate_moves(|moves| {
            for &victim in &Piece::ALL {
                let mut capture_moves = moves;
                capture_moves.to &= their_pieces & board.pieces(victim);
                for mv in capture_moves {
                    let eval = EVALUATOR.piece_value(victim) * Eval::cp(1000)
                        - EVALUATOR.piece_value(moves.piece);
                    let score = if eval >= Eval::ZERO {
                        MoveScore::Capture(eval)
                    } else {
                        MoveScore::LosingCapture(eval)
                    };
                    move_list.push((mv, score));
                }
            }
            false
        });
        MoveList {
            move_list
        }
    }
}

impl Iterator for MoveList {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        let max_index = self.move_list.iter()
            .enumerate()
            .max_by_key(|(_, (_, score))| score)
            .map(|(i, _)| i);
        if let Some(max_index) = max_index {
            Some(self.move_list.swap_remove(max_index).0)
        } else {
            None
        }
    }
}
