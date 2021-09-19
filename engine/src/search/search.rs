use arrayvec::ArrayVec;
use cozy_chess::*;

use crate::eval::*;
use super::SearchHandler;
use super::cache::*;
use super::helpers::move_is_quiet;
use super::window::Window;
use super::oracle;

#[derive(Debug, Clone, Default)]
pub struct SearchStats {
    pub nodes: u64
}

pub struct SearchSharedState<H> {
    pub handler: H,
    pub history: Vec<u64>,
    pub cache_table: CacheTable
}

pub(crate) type KillerEntry = ArrayVec<Move, 2>;
pub(crate) type HistoryTable = [[[u32; Square::NUM]; Piece::NUM]; Color::NUM];

pub struct Searcher<'s, H> {
    pub shared: &'s mut SearchSharedState<H>,
    pub search_result: Option<Move>,
    pub history: Vec<u64>,
    pub killers: Vec<KillerEntry>,
    pub history_table: HistoryTable,
    pub stats: SearchStats
}

macro_rules! node_types {
    ($($type:ident),*) => {
        pub mod node {
            use std::any::TypeId;

            pub trait NodeType: 'static {
                fn is<N: NodeType>() -> bool {
                    TypeId::of::<Self>() == TypeId::of::<N>()
                }
            }

            $(pub struct $type;
    
            impl NodeType for $type {})*
        }
    };
}

node_types! {
    Root,
    Normal
}
use node::NodeType;

const NULL_MOVE_REDUCTION: u8 = 2;
const LMR_MIN_DEPTH: u8 = 3;
fn lmr_calculate_reduction(i: usize) -> u8 {
    if i < 3 {
        0
    } else {
        1
    }
}

impl<H: SearchHandler> Searcher<'_, H> {
    pub fn search_node<Node: NodeType>(
        &mut self,
        board: &Board,
        mut depth: u8,
        ply_index: u8,
        mut window: Window
    ) -> Result<Eval, ()> {
        self.history.push(board.hash());
        let result = (|| {
            let in_check = !board.checkers().is_empty();

            if in_check {
                //Don't enter quiescence while in check
                depth += 1;
            }

            if depth == 0 {
                return Ok(self.quiescence(board, ply_index, window));
            }

            self.stats.nodes += 1;

            let init_window = window;

            if self.shared.handler.stop_search() {
                return Err(());
            }

            if !Node::is::<node::Root>() && self.repetition(&board) {
                return Ok(Eval::DRAW);
            }
            match board.status() {
                GameStatus::Won => return Ok(Eval::mated_in(ply_index)),
                GameStatus::Drawn => return Ok(Eval::DRAW),
                GameStatus::Ongoing => {}
            }
            if !Node::is::<node::Root>() {
                if let Some(eval) = oracle::oracle(&board) {
                    return Ok(eval);
                }
            }

            let mut pv_move = None;
            if let Some(entry) = self.shared.cache_table.get(&board) {
                pv_move = Some(entry.best_move);
                if entry.depth >= depth {
                    match entry.kind {
                        TableEntryKind::Exact => if !Node::is::<node::Root>() {
                            return Ok(entry.eval);
                        },
                        TableEntryKind::LowerBound => window.narrow_alpha(entry.eval),
                        TableEntryKind::UpperBound => window.narrow_beta(entry.eval),
                    }
                    if !Node::is::<node::Root>() && window.empty() {
                        return Ok(entry.eval);
                    }
                }
            }

            let our_pieces = board.colors(board.side_to_move());
            let sliding_pieces =
                board.pieces(Piece::Rook) |
                board.pieces(Piece::Bishop) |
                board.pieces(Piece::Queen);

            let mut best_move = None;
            let mut best_eval = Eval::MIN;
            if !Node::is::<node::Root>() && !(our_pieces & sliding_pieces).is_empty() {
                if let Some(child) = board.null_move() {
                    let mut window = window.null_window_beta();
                    let eval = -self.search_node::<node::Normal>(
                        &child,
                        (depth - 1).saturating_sub(NULL_MOVE_REDUCTION),
                        ply_index + 1,
                        -window
                    )?;
                    window.narrow_alpha(eval);
                    if window.empty() {
                        return Ok(eval);
                    }
                }
            }
            let moves = self.new_movelist(
                board,
                pv_move,
                self.killers[ply_index as usize].clone()
            );
            for (i, mv) in moves.enumerate() {
                let mut child = board.clone();
                child.play_unchecked(mv);
                let gives_check = !child.checkers().is_empty();
                let quiet = move_is_quiet(mv, &board);

                let mut child_window = if i > 0 {
                    window.null_window_alpha()
                } else {
                    window
                };
                let mut reduction = 0;
                if depth > LMR_MIN_DEPTH && quiet && !in_check && !gives_check {
                    reduction += lmr_calculate_reduction(i);
                }
                let mut eval = -self.search_node::<node::Normal>(
                    &child,
                    (depth - 1).saturating_sub(reduction),
                    ply_index + 1,
                    -child_window
                )?;
                if (child_window != window || reduction > 0) && window.contains(eval) {
                    child_window = window;
                    eval = -self.search_node::<node::Normal>(
                        &child,
                        depth - 1,
                        ply_index + 1,
                        -child_window
                    )?;
                }

                if eval > best_eval {
                    best_eval = eval;
                    best_move = Some(mv);
                }

                window.narrow_alpha(eval);
                if window.empty() {
                    if move_is_quiet(mv, &board) {
                        let killers = &mut self.killers[ply_index as usize];
                        if killers.is_full() {
                            killers.remove(0);
                        }
                        killers.push(mv);
                        self.history_table
                            [board.side_to_move() as usize]
                            [board.piece_on(mv.from).unwrap() as usize]
                            [mv.to as usize]
                            += depth as u32 * depth as u32;
                    }
                    break;
                }
            }
            let best_move = best_move.unwrap();

            self.shared.cache_table.set(&board, TableEntry {
                kind: match best_eval {
                    //No move was capable of raising alpha.
                    //The actual value might be worse than this.
                    _ if best_eval <= init_window.alpha => TableEntryKind::UpperBound,
                    //The move was too good and this is a cut node.
                    //The value might be even better if it were not cut off.
                    _ if best_eval >= window.beta => TableEntryKind::LowerBound,
                    //It's in the window. This is an exact value.
                    _ => TableEntryKind::Exact
                },
                eval: best_eval,
                depth,
                best_move
            });

            if Node::is::<node::Root>() {
                self.search_result = Some(best_move);
            }

            Ok(best_eval)
        })();
        self.history.pop();
        result
    }

    fn quiescence(
        &mut self,
        board: &Board,
        ply_index: u8,
        mut window: Window
    ) -> Eval {
        self.history.push(board.hash());
        let result = (|| {
            self.stats.nodes += 1;

            if self.repetition(board) {
                return Eval::DRAW;
            }
            match board.status() {
                GameStatus::Won => return Eval::mated_in(ply_index),
                GameStatus::Drawn => return Eval::DRAW,
                GameStatus::Ongoing => {}
            }
            if let Some(eval) = oracle::oracle(&board) {
                return eval;
            }

            if let Some(entry) = self.shared.cache_table.get(board) {
                match entry.kind {
                    TableEntryKind::Exact => return entry.eval,
                    TableEntryKind::LowerBound => window.narrow_alpha(entry.eval),
                    TableEntryKind::UpperBound => window.narrow_beta(entry.eval),
                }
                if window.empty() {
                    return entry.eval;
                }
            }

            let mut best_eval = EVALUATOR.evaluate(board);
            window.narrow_alpha(best_eval);
            if window.empty() {
                return best_eval;
            }

            for mv in self.quiet_movelist(board) {
                let mut child = board.clone();
                child.play_unchecked(mv);
                let eval = -self.quiescence(
                    &child,
                    ply_index + 1,
                    -window
                );

                if eval > best_eval {
                    best_eval = eval;
                }

                window.narrow_alpha(eval);
                if window.empty() {
                    break;
                }
            }

            best_eval
        })();
        self.history.pop();
        result
    }

    fn repetition(&self, board: &Board) -> bool {
        self.history.iter()
            .rev()
            .take(board.halfmove_clock() as usize)
            .step_by(2) // Every second ply so it's our turn
            .skip(1) // Skip our board
            .any(|&hash| hash == board.hash())
    }
}
