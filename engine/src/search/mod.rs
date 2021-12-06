use std::convert::TryInto;
use std::num::NonZeroU8;

use cozy_chess::*;

use crate::eval::Eval;

mod search;
mod window;
mod cache;
mod moves;
mod helpers;
mod oracle;

use search::*;
use window::Window;
pub use cache::{CacheTable, TableEntry, TableKeyValueEntry};

pub trait SearchHandler {
    fn stop_search(&self) -> bool;
    fn new_result(&mut self, result: SearchResult);
}

impl<H: SearchHandler, R: std::ops::DerefMut<Target=H>> SearchHandler for R {
    fn stop_search(&self) -> bool {
        (**self).stop_search()
    }

    fn new_result(&mut self, search_result: SearchResult) {
        (**self).new_result(search_result)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub mv: Move,
    pub eval: Eval,
    pub nodes: u64,
    pub depth: u8,
    pub seldepth: u8,
    pub used_cache_entries: u32,
    pub total_cache_entries: u32,
    pub principal_variation: Vec<Move>
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub max_depth: NonZeroU8
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            max_depth: 64.try_into().unwrap()
        }
    }
}

pub struct Engine<H> {
    board: Board,
    shared: SearchSharedState<H>,
    options: EngineOptions
}

impl<H: SearchHandler> Engine<H> {
    pub fn new(
        handler: H,
        init_pos: Board,
        moves: impl IntoIterator<Item=Move>,
        options: EngineOptions,
        cache_table: CacheTable
    ) -> Self {
        let mut history = Vec::with_capacity(options.max_depth.get() as usize);
        let mut board = init_pos.clone();
        for mv in moves {
            history.push(board.hash());
            board.play_unchecked(mv);
        }

        Self {
            board,
            shared: SearchSharedState {
                handler,
                history,
                cache_table
            },
            options
        }
    }

    pub fn search(&mut self) {
        const EMPTY_KILLER_ENTRY: KillerEntry = KillerEntry::new_const();
        let mut aspiration_window = Window::INFINITY;
        for depth in 1..=self.options.max_depth.get() {
            let history = self.shared.history.clone();
            let mut searcher = Searcher {
                shared: &mut self.shared,
                search_result: None,
                history,
                killers: [EMPTY_KILLER_ENTRY; u8::MAX as usize],
                history_table: [[[0; Square::NUM]; Piece::NUM]; Color::NUM],
                stats: SearchStats::default()
            };

            let mut eval = searcher.search_node::<node::Root>(
                &self.board,
                depth,
                0,
                aspiration_window
            );
            if let Ok(e) = eval {
                if !aspiration_window.contains(e) {
                    eval = searcher.search_node::<node::Root>(
                        &self.board,
                        depth,
                        0,
                        Window::INFINITY
                    );
                }
            }

            if let Ok(eval) = eval {
                if depth >= 3 {
                    aspiration_window = Window::around(eval, Eval::cp(75));
                }
                let mv = searcher.search_result.unwrap();
                let stats = searcher.stats;
                let mut principal_variation = Vec::new();
                let mut history = self.shared.history.clone();
                let mut board = self.board.clone();
                while let Some(entry) = self.shared.cache_table.get(&board) {
                    history.push(board.hash());
                    board.play_unchecked(entry.best_move);
                    principal_variation.push(entry.best_move);
                    let repetitions = history.iter()
                        .rev()
                        .take(board.halfmove_clock() as usize + 1)
                        .step_by(2) // Every second ply so it's our turn
                        .skip(1)
                        .filter(|&&hash| hash == board.hash())
                        .count();
                    if repetitions > 2 || board.status() != GameStatus::Ongoing {
                        break;
                    }
                }

                self.shared.handler.new_result(SearchResult {
                    mv,
                    eval,
                    nodes: stats.nodes,
                    depth,
                    seldepth: stats.seldepth,
                    used_cache_entries: self.shared.cache_table.len(),
                    total_cache_entries: self.shared.cache_table.capacity(),
                    principal_variation
                });
            } else {
                break;
            }
        }
    }

    pub fn into_cache_table(self) -> CacheTable {
        self.shared.cache_table
    }
}
