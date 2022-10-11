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
    max_eval: i16,

    /// Enable cp prescaling
    #[clap(long, default_value_t = false)]
    prescaling: bool,

    /// Initial WDL weight when prescaling eval
    #[clap(long, default_value_t = 0.0)]
    eval_wdl_prescaling_begin: f32,

    /// Final WDL weight when prescaling eval
    #[clap(long, default_value_t = 0.0)]
    eval_wdl_prescaling_end: f32,

    /// Eval scale factor for WDL conversion
    #[clap(long, default_value_t = 115.0)]
    eval_scale: f32
}

fn cp_to_wdl(cp: f32, scale: f32) -> f32 {
    1.0 / (1.0 + (-cp / scale).exp())
}

fn wdl_to_cp(wdl: f32, scale: f32) -> f32 {
    -scale * (1.0 / wdl - 1.0).ln()
}

fn lerp(a: f32, b: f32, i: f32) -> f32 {
    a * (1.0 - i) + b * i
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
        // TODO better name
        let mut next_board = init_pos.clone();
        for (i, &mv) in game.moves.iter().enumerate() {
            let board = next_board.clone();
            next_board.play_unchecked(mv);

            if i < game.opening_moves as usize {
                continue;
            }
            let mut cp = match game.evals[i - game.opening_moves as usize].as_cp() {
                Some(cp) => cp,
                None => continue,
            };

            let is_capture = next_board.occupied().len() < board.occupied().len();
            if config.exclude_captures && is_capture {
                continue;
            }

            let has_checkers = !board.checkers().is_empty() || !next_board.checkers().is_empty();
            if config.exclude_checkers && has_checkers {
                continue;
            }

            if cp.abs() > config.max_eval {
                continue;
            }

            if config.prescaling {
                let index = i - game.opening_moves as usize;
                let total = game.moves.len() - 1 - game.opening_moves as usize;
                
                let cp_wdl = cp_to_wdl(cp as f32, config.eval_scale);
                let wdl = match game.winner {
                    Some(Color::White) => 1.0,
                    Some(Color::Black) => 0.0,
                    None => 0.5,
                };
                let wdl_weight = lerp(
                    config.eval_wdl_prescaling_begin,
                    config.eval_wdl_prescaling_end,
                    index as f32 / total as f32
                );
                let cp_wdl = lerp(cp_wdl, wdl, wdl_weight);
                cp = wdl_to_cp(cp_wdl, config.eval_scale).round() as i16;
            }

            samples.push((board, cp));
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
