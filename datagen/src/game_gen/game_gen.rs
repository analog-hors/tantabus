use cozy_chess::*;

use rand::prelude::*;
use tantabus::search::*;

use crate::analyzed_game::AnalyzedGame;
use super::chess_game::ChessGame;

fn random_opening(opening_moves: u8) -> ChessGame {
    fn try_random_opening(opening_moves: u8) -> Option<ChessGame> {
        let mut game = ChessGame::new();
        for _ in 0..opening_moves {
            let mut moves = Vec::new();
            game.board().generate_moves(|move_set| {
                moves.extend(move_set);
                false
            });
            let mv = *moves.choose(&mut thread_rng()).unwrap();
            game.play_unchecked(mv);
            if game.game_status() != GameStatus::Ongoing {
                return None;
            }
        }
        Some(game)
    }
    loop {
        if let Some(game) = try_random_opening(opening_moves) {
            return game;
        }
    }
}

struct Handler {
    nodes: u64,
    min_nodes: u64,
    min_depth: u8,
    prev_result: Option<SearchResult>
}

impl SearchHandler for Handler {
    fn stop_search(&self) -> bool {
        if let Some(result) = &self.prev_result {
            result.depth >= self.min_depth && result.nodes >= self.min_nodes
        } else {
            false
        }
    }

    fn new_result(&mut self, search_result: SearchResult) {
        self.nodes += search_result.nodes;
        self.prev_result = Some(search_result);
    }
}

#[derive(Debug, Clone)]
pub struct GameGenConfig {
    pub cache_size: usize,
    pub opening_moves: u8,
    pub min_nodes: u64,
    pub min_depth: u8
}

pub fn gen_game(config: &GameGenConfig) -> AnalyzedGame {
    let init_pos = Board::default();
    let mut game = random_opening(config.opening_moves);
    let mut evals = Vec::new();
    let mut cache_table = CacheTable::new_with_size(config.cache_size).unwrap();
    loop {
        let mut handler = Handler {
            nodes: 0,
            min_nodes: config.min_nodes,
            min_depth: config.min_depth,
            prev_result: None
        };
        let mut engine = Engine::new(
            &mut handler,
            init_pos.clone(),
            game.moves().iter().copied(),
            EngineOptions::default(),
            SearchParams::default(),
            cache_table
        );
        engine.search();
        cache_table = engine.into_cache_table();

        let analysis = handler.prev_result.unwrap();
        evals.push(match game.board().side_to_move() {
            Color::White => analysis.eval,
            Color::Black => -analysis.eval
        });
        game.play_unchecked(analysis.mv);

        let status = game.game_status();
        if status != GameStatus::Ongoing {
            let winner = if status == GameStatus::Won {
                Some(!game.board().side_to_move())
            } else {
                None
            };
            return AnalyzedGame {
                opening_moves: config.opening_moves,
                moves: game.into_moves(),
                evals,
                winner
            };
        }
    }
}
