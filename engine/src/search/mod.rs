use std::convert::TryInto;
use std::num::{NonZeroU8, NonZeroU32};

use cozy_chess::*;

use crate::eval::Eval;
use crate::nnue::Nnue;

mod search;
mod window;
mod cache;
mod moves;
mod helpers;
mod oracle;
mod history;
mod params;
mod position;

use search::*;
pub use params::*;
use window::Window;
pub use cache::{CacheTable, CacheData};
use position::Position;

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
    pub cache_approx_size_permill: u32,
    pub principal_variation: Vec<Move>
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub max_depth: NonZeroU8,
    pub threads: NonZeroU32
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            max_depth: 64.try_into().unwrap(),
            threads: 1.try_into().unwrap(),
        }
    }
}

pub struct Engine<H> {
    pos: Position<'static>,
    main_handler: H,
    shared: SearchSharedState,
    options: EngineOptions
}

impl<H: SearchHandler> Engine<H> {
    pub fn new(
        handler: H,
        init_pos: Board,
        moves: impl IntoIterator<Item=Move>,
        options: EngineOptions,
        search_params: SearchParams,
        cache_table: CacheTable
    ) -> Self {
        let mut history = Vec::with_capacity(options.max_depth.get() as usize);
        let mut board = init_pos;
        for mv in moves {
            history.push(board.hash());
            board.play_unchecked(mv);
        }

        Self {
            pos: Position::new(&Nnue::DEFAULT, board),
            main_handler: handler,
            shared: SearchSharedState {
                history,
                cache_table,
                search_params
            },
            options
        }
    }

    pub fn search(&mut self) {
        let mut prev_eval = None;

        let mut search_data = (0..self.options.threads.get())
            .map(|_| SearchData::new(self.shared.history.clone()))
            .collect::<Vec<_>>();

        for depth in 1..=self.options.max_depth.get() {
            let mut windows = [75].iter().copied().map(Eval::cp);
            let result = loop {
                // CITE: Aspiration window.
                // https://www.chessprogramming.org/Aspiration_Windows
                let mut aspiration_window = Window::INFINITY;
                if depth > 3 {
                    if let Some(prev_eval) = prev_eval {
                        if let Some(bounds) = windows.next() {
                            aspiration_window = Window::around(prev_eval, bounds);
                        }
                    }
                }
                let result = Searcher::search(
                    &mut self.main_handler,
                    &self.shared,
                    &mut search_data[0],
                    &self.pos,
                    depth,
                    aspiration_window
                );
                if let Ok(result) = &result {
                    if !aspiration_window.contains(result.eval) {
                        continue;
                    }
                }
                break result;
            };

            let SearcherResult { mv, eval, stats } = match result {
                Ok(result) => result,
                Err(_) => break
            };

            prev_eval = Some(eval);
            let mut principal_variation = Vec::new();
            let mut history = self.shared.history.clone();
            let mut board = self.pos.board().clone();
            while let Some(entry) = self.shared.cache_table.get(&board, 0) {
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

            self.main_handler.new_result(SearchResult {
                mv,
                eval,
                nodes: stats.nodes,
                depth,
                seldepth: stats.seldepth,
                cache_approx_size_permill: self.shared.cache_table.approx_size_permill(),
                principal_variation
            });
        }
    }

    pub fn into_cache_table(self) -> CacheTable {
        self.shared.cache_table
    }
}
