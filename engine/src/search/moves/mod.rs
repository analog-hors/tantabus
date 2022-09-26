use cozy_chess::*;
use arrayvec::ArrayVec;

use super::search::{KillerEntry, Searcher, KILLER_ENTRIES};

mod see;
mod partition;

use see::*;
use partition::*;

// CITE: Move ordering.
// This move ordering was originally derived from this page:
// https://www.chessprogramming.org/Move_Ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MoveScore {
    LosingCapture(i32),
    Quiet(i32),
    Killer,
    Capture(i32),
    WinningCapture(i32),
    Pv
}

type ScoredMove = (Move, MoveScore);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MoveGenStage {
    Pv,
    Captures,
    Killers,
    Quiets,
    LosingCaptures,
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

// 12 pieces that can capture on 8 squares, 4 pieces that can capture on 4 squares.
// Promotions included.
const MAX_CAPTURES: usize = 12 * 8 + 4 * 4;

struct MoveListData<'b> {
    board: &'b Board,
    pv_move: Option<Move>,
    killers: KillerEntry
}

pub struct MoveList<'b> {
    data: MoveListData<'b>,
    move_list: PartitionedMoveList,
    yielded: usize,
    stage: MoveGenStage,
    pv: Option<Partition>,
    captures: Option<Partition>,
    dense_quiets: ArrayVec<PieceMoves, 18>,
    killers: Option<Partition>,
    quiets: Option<Partition>,
    losing_captures: Option<Partition>,
}

impl<'b> MoveList<'b> {
    pub fn new(board: &'b Board, pv_move: Option<Move>, killers: KillerEntry) -> Self {
        Self {
            data: MoveListData {
                board,
                pv_move,
                killers
            },
            move_list: PartitionedMoveList::new(),
            yielded: 0,
            stage: MoveGenStage::Pv,
            pv: None,
            captures: None,
            dense_quiets: ArrayVec::new(),
            killers: None,
            quiets: None,
            losing_captures: None
        }
    }

    pub fn yielded(&self) -> impl Iterator<Item=&ScoredMove> + '_ {
        let partitions = [
            &self.pv,
            &self.captures,
            &self.killers,
            &self.quiets,
            &self.losing_captures
        ];
        partitions.into_iter()
            .filter_map(|p| p.as_ref())
            .flat_map(|p| self.move_list.yielded_from_partition(p))
    }

    pub fn pick<H>(&mut self, searcher: &Searcher<H>) -> Option<(usize, ScoredMove)> {
        let mv = self.pick_inner(searcher)?;
        let index = self.yielded;
        self.yielded += 1;
        Some((index, mv))
    }

    fn pick_inner<H>(&mut self, searcher: &Searcher<H>) -> Option<ScoredMove> {
        if self.stage == MoveGenStage::Pv {
            if self.pv.is_none() {
                self.pv = Some(self.move_list.new_partition(|mut pv| {
                    if let Some(pv_move) = self.data.pv_move {
                        pv.push((pv_move, MoveScore::Pv));
                    }
                }));
            }
            let pv = self.pv.as_mut().unwrap();
            if let Some(&result) = self.move_list.yield_from_partition(pv) {
                return Some(result);
            }
            self.stage = MoveGenStage::Captures;
        }
        if self.stage == MoveGenStage::Captures {
            if self.captures.is_none() {
                let mut losing_captures = ArrayVec::<_, MAX_CAPTURES>::new();
                self.captures = Some(self.move_list.new_partition(|mut captures| {
                    let their_pieces = self.data.board.colors(!self.data.board.side_to_move());
                    self.data.board.generate_moves(|mut moves| {
                        if let Some(pv_move) = self.data.pv_move {
                            if moves.from == pv_move.from && moves.to.has(pv_move.to) {
                                moves.to ^= pv_move.to.bitboard();
                            }
                        }
                        let mut capture_moves = moves;
                        capture_moves.to &= their_pieces;
                        let mut quiet_moves = moves;
                        quiet_moves.to ^= capture_moves.to;
        
                        if !quiet_moves.is_empty() {
                            self.dense_quiets.push(quiet_moves);
                        }
        
                        for mv in capture_moves {
                            let history = searcher.data.capture_history.get(self.data.board, mv);
                            if static_exchange_evaluation_at_least::<0>(self.data.board, mv) {
                                if static_exchange_evaluation_at_least::<1>(self.data.board, mv) {
                                    captures.push((mv, MoveScore::WinningCapture(history)));
                                } else {
                                    captures.push((mv, MoveScore::Capture(history)));
                                }
                            } else {
                                losing_captures.push((mv, MoveScore::LosingCapture(history)));
                            }
                        }
                        false
                    });
                }));
                self.losing_captures = Some(self.move_list.new_partition_from_slice(&losing_captures));
            }
            let captures = self.captures.as_mut().unwrap();
            if let Some(&result) = self.move_list.yield_from_partition(captures) {
                return Some(result);
            }
            self.stage = MoveGenStage::Killers;
        }
        if self.stage == MoveGenStage::Killers {
            if self.killers.is_none() {
                let mut killers = ArrayVec::<_, KILLER_ENTRIES>::new();
                self.quiets = Some(self.move_list.new_partition(|mut quiets| {
                    for &moves in &self.dense_quiets {
                        for mv in moves {
                            if self.data.killers.contains(&mv) {
                                killers.push((mv, MoveScore::Killer));
                            } else {
                                let history = searcher.data.quiet_history.get(self.data.board, mv);
                                quiets.push((mv, MoveScore::Quiet(history)));
                            }
                        }
                    }
                }));
                self.killers = Some(self.move_list.new_partition_from_slice(&killers));
            }
            let killers = self.killers.as_mut().unwrap();
            if let Some(&result) = self.move_list.yield_from_partition(killers) {
                return Some(result);
            }
            self.stage = MoveGenStage::Quiets;
        }
        if self.stage == MoveGenStage::Quiets {
            let quiets = self.quiets.as_mut().unwrap();
            if let Some(&result) = self.move_list.yield_from_partition(quiets) {
                return Some(result);
            }
            self.stage = MoveGenStage::LosingCaptures;
        }
        if self.stage == MoveGenStage::LosingCaptures {
            let losing_captures = self.losing_captures.as_mut().unwrap();
            if let Some(&result) = self.move_list.yield_from_partition(losing_captures) {
                return Some(result);
            }
            self.stage = MoveGenStage::Finished;
        }
        None
    }
}

pub struct QSearchMoveList {
    move_list: ArrayVec<ScoredMove, MAX_CAPTURES>,
    yielded: usize
}

impl QSearchMoveList {
    pub fn new<H>(board: &Board, searcher: &Searcher<H>) -> Self {
        let mut move_list = ArrayVec::new();

        let their_pieces = board.colors(!board.side_to_move());
        board.generate_moves(|moves| {
            let mut capture_moves = moves;
            capture_moves.to &= their_pieces;
            for mv in capture_moves {
                // CITE: This use of SEE in quiescence and pruning moves with
                // negative SEE was implemented based on a chessprogramming.org page.
                // https://www.chessprogramming.org/Quiescence_Search#Limiting_Quiescence
                if !static_exchange_evaluation_at_least::<0>(board, mv) {
                    continue;
                }
                let history = searcher.data.capture_history.get(board, mv);
                if static_exchange_evaluation_at_least::<1>(board, mv) {
                    move_list.push((mv, MoveScore::WinningCapture(history)));
                } else {
                    move_list.push((mv, MoveScore::Capture(history)));
                }
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
