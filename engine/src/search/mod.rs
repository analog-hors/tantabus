use std::convert::TryInto;
use std::num::NonZeroU8;

use arrayvec::ArrayVec;
use cozy_chess::*;

use crate::eval::Eval;

mod search;
mod window;
mod cache;
mod moves;
mod helpers;

use search::*;
use window::Window;
use cache::CacheTable;

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
    pub used_cache_entries: usize,
    pub total_cache_entries: usize
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub cache_table_size: usize,
    pub max_depth: NonZeroU8
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            cache_table_size: 16_000_000,
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
    pub fn new(handler: H, init_pos: Board, moves: impl IntoIterator<Item=Move>, options: EngineOptions) -> Self {
        let mut history = Vec::with_capacity(options.max_depth.get() as usize);
        let mut board = init_pos.clone();
        for mv in moves {
            history.push(board.hash());
            board.play_unchecked(mv);
        }
        let cache_table = CacheTable::with_rounded_size(options.cache_table_size);

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
        for depth in 1..=self.options.max_depth.get() {
            let history = self.shared.history.clone();
            let mut searcher = Searcher {
                shared: &mut self.shared,
                search_result: None,
                history,
                killers: vec![ArrayVec::new(); self.options.max_depth.get() as usize],
                history_table: [[[0; Square::NUM]; Piece::NUM]; Color::NUM],
                stats: SearchStats::default()
            };

            let eval = searcher.search_node::<node::Root>(
                &self.board,
                depth,
                0,
                Window::INFINITY
            );

            if let Ok(eval) = eval {
                let mv = searcher.search_result.unwrap();
                let stats = searcher.stats;
                self.shared.handler.new_result(SearchResult {
                    mv,
                    eval,
                    nodes: stats.nodes,
                    depth,
                    used_cache_entries: self.shared.cache_table.len(),
                    total_cache_entries: self.shared.cache_table.capacity()
                });
            } else {
                break;
            }
        }
    }
}
