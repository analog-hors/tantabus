use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use clap::Args;

mod chess_game;
mod game_gen;

use game_gen::{GameGenConfig, gen_game};

use crate::analyzed_game::write_analyzed_game;

fn default_threads() -> u32 {
    std::thread::available_parallelism().map_or(1, |t| t.get() as u32)
}

const GAMES_PER_LOG: u64 = 100;

#[derive(Debug, Args)]
/// Generate games
pub struct GameGenRunnerConfig {
    /// The output file
    #[clap(short, long)]
    out_file: PathBuf,

    /// Thread count
    #[clap(long, default_value_t = default_threads())]
    threads: u32,

    /// Cache size in megabytes
    #[clap(long, default_value_t  = 1)]
    cache_size: u32,

    /// Number of randomized opening moves
    #[clap(long, default_value_t = 8)]
    opening_moves: u8,

    /// Minimum node count per move
    #[clap(long, default_value_t = 0)]
    min_nodes: u64,

    /// Minimum depth per move
    #[clap(long, default_value_t = 7)]
    min_depth: u8
}

struct GameGenSharedState {
    out_file: BufWriter<File>,
    last_log: Instant,
    games_written: u64
}

pub fn run_game_gen(config: &GameGenRunnerConfig, abort: &Arc<AtomicBool>) {
    let threads = config.threads;
    let game_gen_config = GameGenConfig {
        cache_size: config.cache_size as usize * 1000_000,
        opening_moves: config.opening_moves,
        min_nodes: config.min_nodes,
        min_depth: config.min_depth
    };
    let out_file = File::options()
        .write(true)
        .create_new(true)
        .open(&config.out_file)
        .expect("Failed to create out file");
    let shared_state = GameGenSharedState {
        out_file: BufWriter::new(out_file),
        last_log: Instant::now(),
        games_written: 0,
    };
    let shared_state = Arc::new(Mutex::new(shared_state));
    let mut thread_handles = Vec::with_capacity(threads as usize);
    for _ in 0..threads {
        let abort = Arc::clone(abort);
        let game_gen_config = game_gen_config.clone();
        let shared_state = Arc::clone(&shared_state);
        let handle = std::thread::spawn(move || {
            while !abort.load(Ordering::SeqCst) {
                let analysis = gen_game(&game_gen_config);
                let mut shared_state = shared_state.lock().unwrap();
                write_analyzed_game(&analysis, &mut shared_state.out_file).unwrap();
                shared_state.games_written += 1;
                if shared_state.games_written % GAMES_PER_LOG == 0 {
                    let now = Instant::now();
                    let elapsed = now.duration_since(shared_state.last_log);
                    shared_state.last_log = now;
                    eprintln!(
                        "{} games written ({:.2} games/s).",
                        shared_state.games_written,
                        GAMES_PER_LOG as f32 / elapsed.as_secs_f32()
                    );
                }
            }
        });
        thread_handles.push(handle);
    }
    for handle in thread_handles {
        handle.join().unwrap();
    }
}
