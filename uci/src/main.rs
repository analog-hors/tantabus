use std::iter::Peekable;
use std::str::SplitAsciiWhitespace;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

use cozy_chess::*;
use tantabus::eval::EvalKind;
use tantabus::search::{SearchResult, CacheTable, SearchHandler, Engine};

mod bench;
mod options;

use options::*;
use tantabus::time::{StandardTimeManager, TimeManager};

const ENGINE_NAME: &str = concat!("Tantabus ", env!("CARGO_PKG_VERSION"));
const ENGINE_AUTHOR: &str = "Analog Hors";

#[derive(Default)]
struct UciSearchControl {
    infinite: bool,
    movetime: Option<Duration>,
    wtime: Option<Duration>,
    btime: Option<Duration>,
    winc: Duration,
    binc: Duration,
    depth: Option<u32>    
}

enum Event {
    UciMessage(String),
    SearchInfo(SearchResult, Duration),
    SearchFinished(SearchResult, CacheTable)
}

type TokenStream<'a> = Peekable<SplitAsciiWhitespace<'a>>;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("bench") {
        bench::bench();
        return;
    }

    let (event_sink, event_source) = channel();
    std::thread::spawn({
        let event_sink = event_sink.clone();
        move || {
            let mut lines = std::io::stdin().lines();
            while let Some(Ok(message)) = lines.next() {
                let _ = event_sink.send(Event::UciMessage(message));
            }
            let _ = event_sink.send(Event::UciMessage("quit".to_owned()));
        }
    });

    let mut position = None;
    let mut options_handler = UciOptionsHandler::new();
    let mut search_terminator = None;
    let mut cache_table = None;

    while let Ok(event) = event_source.recv() {
        match event {
            Event::UciMessage(message) => {
                let mut tokens = message.trim().split_ascii_whitespace().peekable();
                let command = match tokens.next() {
                    Some(c) => c,
                    None => continue
                };
                match command {
                    "uci" => {
                        println!("info name {}", ENGINE_NAME);
                        println!("info author {}", ENGINE_AUTHOR);
                        for (name, option, _) in &options_handler.handlers {
                            print!("option name {} ", name);
                            match option {
                                UciOptionKind::Check {
                                    default
                                } => print!("type check default {}", default),
                                UciOptionKind::Spin {
                                    default, min, max
                                } => print!("type spin default {} min {} max {}", default, min, max),
                            }
                            println!();
                        }
                        println!("uciok");
                    }
                    "debug" => {}
                    "isready" => println!("readyok"),
                    "setoption" => {
                        expect(&mut tokens, Some("name"));
                        let name = tokens.next().expect("expected option name in setoption");
                        expect(&mut tokens, Some("value"));
                        let value = tokens.next().expect("expected option value in setoption");
                        options_handler.update(name, value);
                    }
                    "register" => {}
                    "ucinewgame" => cache_table = None,
                    "position" => {
                        let board = match tokens.next() {
                            Some("fen") => {
                                let mut fen = String::new();
                                for i in 0..6 {
                                    let part = tokens.next().expect("expected fen part");
                                    if i == 0 {
                                        fen.push(' ');
                                    }
                                    fen.push_str(part);
                                }
                                fen.parse().expect("invalid fen")
                            }
                            Some("startpos") => Board::default(),
                            unexpected => panic!("unexpected token {:?} (expected \"fen\" or \"startpos\")", unexpected)
                        };
                        let mut moves = Vec::new();
                        if maybe_expect(&mut tokens, "moves") {
                            let mut current = board.clone();
                            while let Some(mv) = tokens.next() {
                                let mv = mv.parse().expect("invalid move");
                                let mv = uci_move_to_move(mv, &current, options_handler.options.chess960);
                                current.try_play(mv).expect("illegal move");
                                moves.push(mv);
                            }
                        }
                        position = Some((board, moves));
                    }
                    "go" => {
                        let search_control = read_uci_search_control(&mut tokens);
                        let time_manager = if search_control.infinite {
                            StandardTimeManager::Infinite
                        } else if let Some(movetime) = search_control.movetime {
                            StandardTimeManager::Fixed(movetime)
                        } else {
                            let (init_pos, moves) = position.as_ref().unwrap();
                            let side_to_move = if moves.len() % 2 == 0 {
                                init_pos.side_to_move()
                            } else {
                                !init_pos.side_to_move()
                            };
                            let (time_left, increment) = match side_to_move {
                                Color::White => (search_control.wtime.unwrap(), search_control.winc),
                                Color::Black => (search_control.btime.unwrap(), search_control.binc),
                            };
                            StandardTimeManager::standard(time_left, increment)
                        };

                        let (init_pos, moves) = position.clone().unwrap();
                        let terminator = Arc::new(AtomicBool::new(false));
                        let mut handler = UciEngineHandler {
                            time_manager,
                            search_begin: Instant::now(),
                            last_update: Instant::now(),
                            time_left: Duration::MAX,
                            search_terminator: terminator.clone(),
                            event_sink: event_sink.clone(),
                            total_nodes: 0,
                            prev_result: None,
                        };
                        std::thread::spawn({
                            let cache_table_size = options_handler.options.cache_table_size;
                            let cache_table = match cache_table.take() {
                                Some(c) => c,
                                None => CacheTable::new_with_size(cache_table_size).unwrap()
                            };

                            let engine_options = options_handler.options.engine_options.clone();
                            let search_params = options_handler.options.search_params.clone();
                            move || {
                                let mut search_state = Engine::new(
                                    &mut handler,
                                    init_pos,
                                    moves,
                                    engine_options,
                                    search_params,
                                    cache_table
                                );
                                search_state.search();
                                let cache_table = search_state.into_cache_table();
                                handler.finish(cache_table);
                            }
                        });
                        search_terminator = Some(terminator);
                    }
                    "stop" => {
                        if let Some(search_terminator) = &mut search_terminator {
                            search_terminator.store(true, Ordering::Release);
                        }
                    }
                    "ponderhit" => {}
                    "quit" => break,
                    _ => println!("info string [WARN] Unknown command {:?} was ignored.", command)
                }
                expect(&mut tokens, None);
            }
            Event::SearchInfo(result, duration) => {
                print!("info");
                match result.eval.kind() {
                    EvalKind::Centipawn(cp) => print!(" score cp {}", cp),
                    EvalKind::MateIn(m) => print!(" score mate {}", (m + 1) / 2),
                    EvalKind::MatedIn(m) => print!(" score mate -{}", (m + 1) / 2)
                }
                print!(
                    " depth {} seldepth {} nodes {} time {} hashfull {}",
                    result.depth,
                    result.seldepth,
                    result.nodes,
                    duration.as_millis(),
                    result.cache_approx_size_permill
                );

                if !result.principal_variation.is_empty() {
                    print!(" pv");
                    let (board, moves) = position.as_ref().unwrap();
                    let mut current_pos = board.clone();
                    for &mv in moves {
                        current_pos.play_unchecked(mv);
                    }
                    for mv in result.principal_variation {
                        let uci_mv = move_to_uci_move(mv, &current_pos, options_handler.options.chess960);
                        print!(" {}", uci_mv);
                        current_pos.play_unchecked(mv);
                    }
                }
                println!();
            }
            Event::SearchFinished(result, cache) => {
                cache_table = Some(cache);
                let (board, moves) = position.as_ref().unwrap();
                let mut current_pos = board.clone();
                for &mv in moves {
                    current_pos.play_unchecked(mv);
                }
                let mv = move_to_uci_move(result.mv, &current_pos, options_handler.options.chess960);
                search_terminator = None;
                println!("bestmove {}", mv);
            }
        }
    }
}

fn expect<'a>(tokens: &mut TokenStream, expected: Option<&str>) {
    let token = tokens.next();
    if token != expected {
        panic!("unexpected token {:?} (expected {:?})", token, expected);
    }
}

fn maybe_expect<'a>(tokens: &mut TokenStream, expected: &str) -> bool {
    let token = tokens.next();
    if let Some(token) = token {
        if token != expected {
            panic!("unexpected token {:?} (expected {:?})", token, expected);
        }
        true
    } else {
        false
    }
}

fn read_uci_search_control<'a>(tokens: &mut TokenStream) -> UciSearchControl {
    let mut search_control = UciSearchControl::default();
    while let Some(token) = tokens.next() {
        match token {
            "searchmoves" => {
                while let Some(token) = tokens.peek() {
                    if token.parse::<Move>().is_ok() {
                        tokens.next();
                    }
                }
                println!("info string [WARN] The searchmoves search control is unimplemented.");
            }
            "ponder" => panic!("ponder is unimplemented"),
            "wtime" => {
                let time = tokens.next().expect("expected time after wtime");
                let time = time.parse().expect("invalid value for wtime");
                search_control.wtime = Some(Duration::from_millis(time));
            }
            "btime" => {
                let time = tokens.next().expect("expected time after btime");
                let time = time.parse().expect("invalid value for btime");
                search_control.btime = Some(Duration::from_millis(time));
            }
            "winc" => {
                let time = tokens.next().expect("expected time after winc");
                let time = time.parse().expect("invalid value for winc");
                search_control.winc = Duration::from_millis(time);
            }
            "binc" => {
                let time = tokens.next().expect("expected time after binc");
                let time = time.parse().expect("invalid value for binc");
                search_control.binc = Duration::from_millis(time);
            }
            "depth" => {
                let plies = tokens.next().expect("expected plies after depth");
                let plies = plies.parse().expect("invalid value for depth");
                search_control.depth = Some(plies);
            }
            "nodes" => {
                let nodes = tokens.next().expect("expected nodes after nodes");
                let _nodes: u64 = nodes.parse().expect("invalid value for nodes");
                println!("info string [WARN] The nodes search control is unimplemented.");
            }
            "mate" => {
                let moves = tokens.next().expect("expected plies after mate");
                let _moves: i32 = moves.parse().expect("invalid value for depth");
                println!("info string [WARN] The mate search control is unimplemented.");
            }
            "movetime" => {
                let time = tokens.next().expect("expected time after movetime");
                let time = time.parse().expect("invalid value for movetime");
                search_control.movetime = Some(Duration::from_millis(time));
            }
            "infinite" => search_control.infinite = true,
            _ => panic!("invalid search control {:?}", token)
        }
    }
    search_control
}

fn uci_move_to_move(mut mv: Move, board: &Board, chess960: bool) -> Move {
    let convert_castle = !chess960
        && board.piece_on(mv.from) == Some(Piece::King)
        && mv.from.file() == File::E
        && matches!(mv.to.file(), File::C | File::G);
    if convert_castle {
        let file = if mv.to.file() == File::C {
            File::A
        } else {
            File::H
        };
        mv.to = Square::new(file, mv.to.rank());
    }
    mv
}

fn move_to_uci_move(mut mv: Move, board: &Board, chess960: bool) -> Move {
    if !chess960 && board.color_on(mv.from) == board.color_on(mv.to) {
        let rights = board.castle_rights(board.side_to_move());
        let file = if Some(mv.to.file()) == rights.short {
            File::G
        } else {
            File::C
        };
        mv.to = Square::new(file, mv.to.rank());
    }
    mv
}

struct UciEngineHandler {
    time_manager: StandardTimeManager,
    search_begin: Instant,
    last_update: Instant,
    time_left: Duration,
    search_terminator: Arc<AtomicBool>,
    event_sink: Sender<Event>,
    total_nodes: u64,
    prev_result: Option<SearchResult>
}

impl SearchHandler for UciEngineHandler {
    fn stop_search(&self) -> bool {
        self.time_left < self.last_update.elapsed() || self.search_terminator.load(Ordering::Acquire)
    }

    fn new_result(&mut self, mut result: SearchResult) {
        self.time_left = self.time_manager.update(&result, self.last_update.elapsed());
        self.last_update = Instant::now();
        self.prev_result = Some(result.clone());
        self.total_nodes += result.nodes;
        result.nodes = self.total_nodes;

        let event = Event::SearchInfo(result, self.search_begin.elapsed());
        self.event_sink.send(event).unwrap();
    }
}

impl UciEngineHandler {
    fn finish(mut self, cache_table: CacheTable) {
        let result = self.prev_result.take().unwrap();
        let event = Event::SearchFinished(result, cache_table);
        self.event_sink.send(event).unwrap();
    }
}
