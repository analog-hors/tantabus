use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::Args;
use cozy_chess::*;
use cozy_syzygy::{Tablebase, Wdl};
use tantabus::eval::Eval;

use crate::analyzed_game::{read_analyzed_game, write_analyzed_game};

#[derive(Debug, Args)]
/// Apply syzygy evals to a set of games
pub struct ApplySyzygyConfig {
    /// The path to the syzygy tablebases
    #[clap(short, long)]
    syzygy_directory: PathBuf,

    /// The input file
    #[clap(short, long)]
    in_file: PathBuf,

    /// The output file
    #[clap(short, long)]
    out_file: PathBuf,

    /// Magnitude of the eval for a win or a loss
    #[clap(long, default_value_t = 20_000)]
    win_score: i16,
    
    /// Magnitude of the eval for a cursed win or a blessed loss
    #[clap(long, default_value_t = 0)]
    cursed_win_score: i16
}

pub fn run_apply_syzygy(config: &ApplySyzygyConfig, abort: &Arc<AtomicBool>) {
    let init_pos = Board::default();

    let mut tablebase = Tablebase::new();
    tablebase.add_directory(&config.syzygy_directory).expect("Failed to add syzygy tablebases");

    let in_file = File::open(&config.in_file).expect("Failed to open in file");
    let out_file = File::options()
        .write(true)
        .create_new(true)
        .open(&config.out_file)
        .expect("Failed to create out file");
    let mut in_file = BufReader::new(in_file);
    let mut out_file = BufWriter::new(out_file);

    while let Some(mut game) = read_analyzed_game(&mut in_file).unwrap() {
        let mut board = init_pos.clone();
        for i in 0..game.moves.len() {
            if i > game.opening_moves as usize && board.occupied().len() <= tablebase.max_pieces() {
                if let Some((wdl, _)) = tablebase.probe_wdl(&board) {
                    let score = match wdl {
                        Wdl::Loss => -config.win_score,
                        Wdl::BlessedLoss => -config.cursed_win_score,
                        Wdl::Draw => 0,
                        Wdl::CursedWin => config.cursed_win_score,
                        Wdl::Win => config.win_score,
                    };
                    let sign = if board.side_to_move() == Color::White { 1 } else { -1 };
                    game.evals[i - game.opening_moves as usize] = Eval::cp(score * sign);
                }
            }
            board.play_unchecked(game.moves[i]);
        }
        write_analyzed_game(&game, &mut out_file).unwrap();

        if abort.load(Ordering::SeqCst) {
            break;
        }
    }
}
