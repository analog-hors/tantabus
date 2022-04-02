use std::io::{stdin, stdout, Write, BufRead};
use std::env::args;
use std::time::Instant;

use rayon::prelude::*;

use cozy_chess::*;
use tantabus::nnue::feature;

mod analyze;

const SCALE: f32 = 115.0;

fn sigmoid(n: f32) -> f32 {
    1.0 / (1.0 + (-n).exp())
}

fn main() {
    let mut stdout = stdout();
    let stdin = stdin();
    let min_nodes = args().nth(1).expect("Expected min nodes").parse().expect("Invalid nodes");
    let lines = stdin.lock().lines().map(Result::unwrap);
    let mut boards = lines.map(|f| f.parse::<Board>().unwrap());
    let mut written = 0;
    loop {
        let start = Instant::now();
        let boards = (&mut boards).take(1024).collect::<Vec<_>>();
        if boards.len() == 0 {
            break;
        }
        let boards = boards
            .into_par_iter()
            .filter_map(|board| to_data(board, min_nodes))
            .collect::<Vec<_>>();
        for (board, win_rate) in &boards {
            write_features(&mut stdout, board, *win_rate);
        }
        let elapsed = start.elapsed();
        let speed = boards.len() as f32 / elapsed.as_secs_f32();
        written += boards.len();
        eprintln!("{} boards written at {} pos/sec", written, speed.round());
    }
}

fn to_data(board: Board, min_nodes: u64) -> Option<(Board, f32)> {
    if board.status() != GameStatus::Ongoing {
        return None;
    }
    let analysis = analyze::analyze(board.clone(), min_nodes);
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

fn write_features(out: &mut impl Write, board: &Board, win_rate: f32) {
    const MAX_FEATURES: u32 = 32;
    for &perspective in &[board.side_to_move(), !board.side_to_move()] {
        let mut features_written = 0;
        for &color in &Color::ALL {
            let colors = board.colors(color);
            for &piece in &Piece::ALL {
                let pieces = board.pieces(piece);
                for square in pieces & colors {
                    let feature = feature(perspective, color, piece, square);
                    out.write_all(&(feature as u16).to_le_bytes()).unwrap();
                    features_written += 1;
                }
            }
        }
        for _ in features_written..MAX_FEATURES {
            out.write_all(&u16::MAX.to_le_bytes()).unwrap();
        }
    }
    out.write_all(&[(win_rate * u8::MAX as f32).round() as u8]).unwrap();
}
