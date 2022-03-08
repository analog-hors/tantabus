use std::io::{stdin, stdout, Write, BufRead};

use cozy_chess::*;
use tantabus::nnue::feature;

const SCALE: f32 = 115.0;

fn sigmoid(n: f32) -> f32 {
    1.0 / (1.0 + (-n).exp())
}

fn main() {
    let mut stdout = stdout();
    for line in stdin().lock().lines().map(Result::unwrap) {
        let (fen, eval) = line.trim().split_once(" | ").unwrap();
        let board = fen.parse().unwrap();
        let eval = eval.parse::<f32>().unwrap();
        let win_rate = sigmoid(eval / SCALE);
        write_features(&mut stdout, &board, win_rate);
    }
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
