use std::fs::File;
use std::io::{Write, BufReader, BufWriter};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rand::prelude::*;
use rand_pcg::Pcg64Mcg;
use clap::{Args, ValueEnum};
use cozy_chess::*;

use crate::analyzed_game::read_analyzed_game;

use marlinformat::write_as_marlinformat;

mod marlinformat;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PositionFormat {
    MarlinFormat,
    FenCpWdl
}

#[derive(Debug, Args)]
/// Extract positions from analyzed games
pub struct ExtractPositionsConfig {
    /// The input file
    #[clap(short, long)]
    in_file: PathBuf,

    /// The output file
    #[clap(short, long)]
    out_file: PathBuf,

    /// The output format
    #[clap(long, value_enum, value_parser, default_value_t = PositionFormat::MarlinFormat)]
    format: PositionFormat,

    /// Max sample size per game
    #[clap(long, default_value_t = 9999)]
    max_samples: u32,

    /// Exclude positions reached from captures
    #[clap(long, default_value_t = true)]
    exclude_captures: bool,

    /// Exclude positions with checkers
    #[clap(long, default_value_t = true)]
    exclude_checkers: bool,

    /// Max absolute eval to be included
    #[clap(long, default_value_t = 20_000)]
    max_eval: i16
}

pub fn run_position_extraction(config: &ExtractPositionsConfig, abort: &Arc<AtomicBool>) {
    let init_pos = Board::default();
    let in_file = File::open(&config.in_file).expect("Failed to open in file");
    let out_file = File::options()
        .write(true)
        .create_new(true)
        .open(&config.out_file)
        .expect("Failed to create out file");
    let mut in_file = BufReader::new(in_file);
    let mut out_file = BufWriter::new(out_file);
    let mut rng = Pcg64Mcg::new(0xcafef00dd15ea5e5);
    while let Some(game) = read_analyzed_game(&mut in_file).unwrap() {
        let mut samples = Vec::new();
        let mut board = init_pos.clone();
        for (i, &mv) in game.moves.iter().enumerate() {
            let piece_count_before_move = board.occupied().popcnt();
            board.play_unchecked(mv);
            if i < game.opening_moves as usize {
                continue;
            }
            let cp = match game.evals[i - game.opening_moves as usize].as_cp() {
                Some(cp) => cp,
                None => continue,
            };

            let is_capture = board.occupied().popcnt() < piece_count_before_move;
            if config.exclude_captures && is_capture {
                continue;
            }

            let has_checkers = !board.checkers().is_empty();
            if config.exclude_checkers && has_checkers {
                continue;
            }

            if cp.abs() > config.max_eval {
                continue;
            }

            samples.push((board.clone(), cp));
        }

        // Partially shuffle into starting elements
        for i in 0..samples.len().min(config.max_samples as usize) {
            let random = rng.gen_range(i..samples.len());
            samples.swap(i, random);
        }
        samples.truncate(config.max_samples as usize);

        match config.format {
            PositionFormat::MarlinFormat => {
                for (board, cp) in samples {
                    write_as_marlinformat(&mut out_file, &board, cp, game.winner).unwrap();
                }
            }
            PositionFormat::FenCpWdl => {
                for (board, cp) in samples {
                    let wdl = match game.winner {
                        Some(Color::White) => "1.0",
                        Some(Color::Black) => "0.0",
                        None => "0.5",
                    };
                    writeln!(&mut out_file, "{} | {} | {}", board, cp, wdl).unwrap();
                }
            }
        }

        if abort.load(Ordering::SeqCst) {
            break;
        }
    }
}
