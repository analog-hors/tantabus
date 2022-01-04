use cozy_chess::*;
use arrayvec::ArrayVec;

use crate::eval::{EVALUATOR, Eval};

use super::SearchHandler;
use super::search::{KillerEntry, Searcher};

fn static_exchange_evaluation(board: &Board, capture: Move) -> Eval {
    fn get_both_pawn_attacks(sq: Square) -> BitBoard {
        get_pawn_attacks(sq, Color::White) | get_pawn_attacks(sq, Color::Black)
    }

    macro_rules! pieces {
        ($($piece:ident)|+) => {
            ($(board.pieces(Piece::$piece))|*)
        }
    }

    let sq = capture.to;
    let mut attacker_sq = capture.from;
    let mut victim = board.piece_on(sq).unwrap();
    let mut attacker = board.piece_on(attacker_sq).unwrap();
    let mut color = board.side_to_move();
    let mut blockers = board.occupied();
    let mut attackers =
        get_king_moves(sq)                   & pieces!(King)           |
        get_knight_moves(sq)                 & pieces!(Knight)         |
        get_rook_moves(sq, blockers)         & pieces!(Rook | Queen)   |
        get_bishop_moves(sq, blockers)       & pieces!(Bishop | Queen) |
        get_both_pawn_attacks(sq) & blockers & pieces!(Pawn);

    //32 pieces max on a legal chess board.
    let mut captures = ArrayVec::<Eval, 32>::new();
    'exchange: loop {
        //"Capture" victim
        captures.push(EVALUATOR.piece_value(victim));

        //"Move" attacker to target square
        let attacker_bitboard = attacker_sq.bitboard();
        blockers ^= attacker_bitboard;
        attackers ^= attacker_bitboard;

        //Add new exposed sliding pieces
        if matches!(attacker, Piece::Rook | Piece::Queen) {
            attackers |= get_rook_moves(sq, blockers) & blockers & pieces!(Rook | Queen);
        }
        if matches!(attacker, Piece::Pawn | Piece::Bishop | Piece::Queen) {
            attackers |= get_bishop_moves(sq, blockers) & blockers & pieces!(Bishop | Queen);
        }

        //Swap sides
        color = !color;

        //Try to fetch a new attacker
        for &new_attacker in &Piece::ALL {
            let mut attackers = attackers &
                board.pieces(new_attacker) &
                board.colors(color);
            if let Some(sq) = attackers.next() {
                if victim == Piece::King {
                    //Oops! Our last capture with our king was illegal since this piece is defended.
                    captures.pop();
                    break;
                }

                //New attacker, the old attacker is now the victim
                victim = attacker;
                attacker = new_attacker;
                attacker_sq = sq;
                continue 'exchange;
            }
        }

        //No attacker could be found, calculate final result.
        while captures.len() > 1 {
            //First capture is forced, but all others can be ignored.
            let forced = captures.len() == 2;
            let their_eval = captures.pop().unwrap();
            let our_capture = captures.last_mut().unwrap();
            *our_capture -= their_eval;
            if !forced && *our_capture < Eval::ZERO {
                //Choose not to make the capture.
                *our_capture = Eval::ZERO;
            }
        }
        return captures.pop().unwrap();
    }
}

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
                        [mv.from as usize]
                        [mv.to as usize];
                    MoveScore::Quiet(history)
                };
                move_list.push((mv, score));
            }

            for mv in capture_moves {
                let eval = static_exchange_evaluation(board, mv);
                let score = if eval >= Eval::ZERO {
                    MoveScore::Capture(eval)
                } else {
                    MoveScore::LosingCapture(eval)
                };
                move_list.push((mv, score));
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
            let mut capture_moves = moves;
            capture_moves.to &= their_pieces;
            for mv in capture_moves {
                let eval = static_exchange_evaluation(board, mv);
                if eval < Eval::ZERO {
                    continue;
                }
                move_list.push((mv, MoveScore::Capture(eval)));
            }
            false
        });
        MoveList {
            move_list
        }
    }
}

impl Iterator for MoveList {
    type Item = (Move, MoveScore);

    fn next(&mut self) -> Option<Self::Item> {
        let max_index = self.move_list.iter()
            .enumerate()
            .max_by_key(|(_, (_, score))| score)
            .map(|(i, _)| i);
        if let Some(max_index) = max_index {
            Some(self.move_list.swap_remove(max_index))
        } else {
            None
        }
    }
}
