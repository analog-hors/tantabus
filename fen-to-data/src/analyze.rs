use cozy_chess::*;
use tantabus::search::*;

const SCALE: f32 = 115.0;

fn sigmoid(n: f32) -> f32 {
    1.0 / (1.0 + (-n).exp())
}

const CACHE: usize = 1_000_000;

struct Handler {
    nodes: u64,
    min_nodes: u64,
    min_depth: u8,
    prev_result: Option<SearchResult>
}

impl SearchHandler for Handler {
    fn stop_search(&self) -> bool {
        self.prev_result
            .as_ref()
            .map(|r| r.depth >= self.min_depth && r.nodes >= self.min_nodes)
            .unwrap_or(false)
    }

    fn new_result(&mut self, search_result: SearchResult) {
        self.nodes += search_result.nodes;
        self.prev_result = Some(search_result);
    }
}

pub struct Analyzer {
    cache: Option<CacheTable>,
    min_nodes: u64,
    min_depth: u8
}

impl Analyzer {
    pub fn new(min_nodes: u64, min_depth: u8) -> Self {
        Self {
            cache: Some(CacheTable::new_with_size(CACHE).unwrap()),
            min_nodes,
            min_depth
        }
    }

    fn analyze(&mut self, board: Board) -> SearchResult {    
        let mut handler = Handler {
            nodes: 0,
            min_nodes: self.min_nodes,
            min_depth: self.min_depth,
            prev_result: None
        };
        let mut engine = Engine::new(
            &mut handler,
            board,
            [],
            EngineOptions::default(),
            SearchParams::default(),
            self.cache.take().unwrap()
        );
        engine.search();
        let mut cache = engine.into_cache_table();
        cache.clear();
        self.cache = Some(cache);
        handler.prev_result.unwrap()
    }

    pub fn to_data(&mut self, board: Board) -> Option<(Board, f32)> {
        if board.status() != GameStatus::Ongoing {
            return None;
        }
        let analysis = self.analyze(board.clone());
        let mut capture_squares = board.colors(!board.side_to_move());
        if let Some(ep) = board.en_passant() {
            let ep = Square::new(ep, Rank::Third.relative_to(!board.side_to_move()));
            capture_squares |= ep.bitboard();
        }
        let is_quiet = board.checkers().is_empty()
            && !capture_squares.has(analysis.mv.to)
            && analysis.eval.as_cp().is_some();
        if !is_quiet {
            return None;
        }
        let eval = analysis.eval.as_cp().unwrap() as f32;
        let win_rate = sigmoid(eval / SCALE);
        Some((board, win_rate))
    }
}
