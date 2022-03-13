use cozy_chess::*;
use tantabus::search::*;

const CACHE: usize = 16_000_000;

struct Handler {
    nodes: u64,
    node_limit: u64,
    prev_result: Option<SearchResult>
}

impl SearchHandler for Handler {
    fn stop_search(&self) -> bool {
        self.prev_result.as_ref().map(|r| r.nodes >= self.node_limit).unwrap_or(false)
    }

    fn new_result(&mut self, search_result: SearchResult) {
        self.nodes += search_result.nodes;
        self.prev_result = Some(search_result);
    }
}

pub fn analyze(board: Board, node_limit: u64) -> SearchResult {    
    let mut handler = Handler {
        nodes: 0,
        node_limit,
        prev_result: None
    };
    let mut engine = Engine::new(
        &mut handler,
        board,
        [],
        EngineOptions::default(),
        SearchParams::default(),
        CacheTable::new_with_size(CACHE).unwrap()
    );
    engine.search();
    handler.prev_result.unwrap()
}
