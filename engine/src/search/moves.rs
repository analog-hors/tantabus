use cozy_chess::*;
use arrayvec::ArrayVec;

use crate::eval::*;

use super::search::{KillerEntry, Searcher};

// CITE: Static exchange evaluation.
// https://www.chessprogramming.org/Static_Exchange_Evaluation
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
        captures.push(Eval::cp(*PIECE_VALUES.get(victim)));

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

// CITE: Move ordering.
// This move ordering was originally derived from this page:
// https://www.chessprogramming.org/Move_Ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveScore {
    LosingCapture(Eval),
    Quiet(i32),
    Killer,
    Capture(Eval),
    Pv
}

type ScoredMove = (Move, MoveScore);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MoveGenStage {
    Pv,
    Remaining,
    Finished
}

fn swap_max_move_to_front(moves: &mut [ScoredMove]) -> Option<&ScoredMove> {
    let max_index = moves
        .iter()
        .enumerate()
        .max_by_key(|(_, (_, score))| score)
        .map(|(i, _)| i);
    if let Some(max_index) = max_index {
        moves.swap(max_index, 0);
    }
    moves.first()
}

pub struct MoveList<'b> {
    board: &'b Board,
    move_list: ArrayVec<ScoredMove, 218>,
    yielded: usize,
    stage: MoveGenStage,
    pv_move: Option<Move>,
    killers: KillerEntry
}

impl<'b> MoveList<'b> {
    pub fn new(board: &'b Board, pv_move: Option<Move>, killers: KillerEntry) -> Self {
        Self {
            board,
            move_list: ArrayVec::new(),
            yielded: 0,
            stage: MoveGenStage::Pv,
            pv_move,
            killers
        }
    }

    pub fn yielded(&self) -> &[ScoredMove] {
        &self.move_list[..self.yielded]
    }

    pub fn pick<H>(&mut self, searcher: &Searcher<H>) -> Option<(usize, ScoredMove)> {
        if self.yielded >= self.move_list.len() && self.stage == MoveGenStage::Pv {
            if let Some(pv_move) = self.pv_move {
                self.move_list.push((pv_move, MoveScore::Pv));
            }
            self.stage = MoveGenStage::Remaining;
        }
        if self.yielded >= self.move_list.len() && self.stage == MoveGenStage::Remaining {
            let their_pieces = self.board.colors(!self.board.side_to_move());
            self.board.generate_moves(|mut moves| {
                if let Some(pv_move) = self.pv_move {
                    if moves.from == pv_move.from && moves.to.has(pv_move.to) {
                        moves.to ^= pv_move.to.bitboard();
                    }
                }
                let mut capture_moves = moves;
                capture_moves.to &= their_pieces;
                let mut quiet_moves = moves;
                quiet_moves.to ^= capture_moves.to;

                for mv in quiet_moves {
                    let score = if self.killers.contains(&mv) {
                        MoveScore::Killer
                    } else {
                        let history = searcher.data.history_table.get(self.board.side_to_move(), mv);
                        MoveScore::Quiet(history)
                    };
                    self.move_list.push((mv, score));
                }

                for mv in capture_moves {
                    let eval = static_exchange_evaluation(self.board, mv);
                    let score = if eval >= Eval::ZERO {
                        MoveScore::Capture(eval)
                    } else {
                        MoveScore::LosingCapture(eval)
                    };
                    self.move_list.push((mv, score));
                }
                false
            });
            self.stage = MoveGenStage::Finished;
        }

        let to_yield = &mut self.move_list[self.yielded..];
        if let Some(&result) = swap_max_move_to_front(to_yield) {
            let index = self.yielded;
            self.yielded += 1;
            return Some((index, result));
        }
        None
    }
}

// 12 pieces that can capture on 8 squares, 4 pieces that can capture on 4 squares.
// 12 * 8 + 4 * 4 = 112
// Promotions included.
pub struct QSearchMoveList {
    move_list: ArrayVec<ScoredMove, 112>,
    yielded: usize
}

impl QSearchMoveList {
    pub fn new(board: &Board) -> Self {
        let mut move_list = ArrayVec::new();

        let their_pieces = board.colors(!board.side_to_move());
        board.generate_moves(|moves| {
            let mut capture_moves = moves;
            capture_moves.to &= their_pieces;
            for mv in capture_moves {
                // CITE: This use of SEE in quiescence and pruning moves with
                // negative SEE was implemented based on a chesspgoramming.org page.
                // https://www.chessprogramming.org/Quiescence_Search#Limiting_Quiescence
                let eval = static_exchange_evaluation(board, mv);
                if eval < Eval::ZERO {
                    continue;
                }
                move_list.push((mv, MoveScore::Capture(eval)));
            }
            false
        });
        Self {
            move_list,
            yielded: 0
        }
    }

    pub fn pick(&mut self) -> Option<(usize, ScoredMove)> {
        let to_yield = &mut self.move_list[self.yielded..];
        if let Some(&result) = swap_max_move_to_front(to_yield) {
            let index = self.yielded;
            self.yielded += 1;
            return Some((index, result));
        }
        None
    }
}
