use std::io::{stdin, stdout, Write, BufRead, BufWriter};
use std::env::args;
use std::str::FromStr;
use std::sync::mpsc::sync_channel;
use std::thread::spawn;
use std::time::Instant;

use cozy_chess::*;

pub fn feature(perspective: Color, mut color: Color, piece: Piece, mut square: Square) -> usize {
    if perspective == Color::Black {
        square = square.flip_rank();
        color = !color;
    }
    macro_rules! index {
        ($([$index:expr; $count:expr])*) => {{
            let mut index = 0;
            $(index = index * $count + $index;)*
            index
        }}
    }
    index! {
        [color as usize; Color::NUM]
        [piece as usize; Piece::NUM]
        [square as usize; Square::NUM]
    }
}

mod analyze;

use analyze::Analyzer;

fn arg<T: FromStr>(n: usize, name: &str) -> T {
    args()
        .nth(n).unwrap_or_else(|| panic!("Expected {} (arg {})", name, n))
        .parse().unwrap_or_else(|_| panic!("Invalid {} (arg {})", name, n))
}

fn main() {
    let threads: u32 = arg(1, "threads");
    let min_nodes = arg(2, "min nodes");
    let min_depth = arg(3, "min depth");

    let (output_send, output_recv) = sync_channel(threads as usize * 2);
    for _ in 0..threads {
        let mut analyzer = Analyzer::new(min_nodes, min_depth);
        let output_send = output_send.clone();
        spawn(move || loop {
            let boards = read_boards();
            if boards.is_empty() {
                break;
            }
            let boards: Vec<_> = boards
                .into_iter()
                .filter_map(|b| analyzer.to_data(b))
                .collect();
            output_send.send(boards).unwrap();
        });
    }
    drop(output_send);

    let stdout = stdout();
    let mut stdout = BufWriter::new(stdout.lock());
    let mut total_written = 0;
    let mut last_printed = Instant::now();
    let mut written_since = 0;
    for batch in output_recv {
        total_written += batch.len();
        written_since += batch.len();
        for (board, win_rate) in batch {
            write_features(&mut stdout, &board, win_rate);
        }
        let elapsed = last_printed.elapsed();
        if elapsed.as_secs() >= 5 {
            let speed = written_since as f32 / elapsed.as_secs_f32();
            eprintln!("{} positions written at {} pos/s", total_written, speed.round());
            last_printed = Instant::now();
            written_since = 0;
        }
    }
}

fn read_boards() -> Vec<Board> {
    let stdin = stdin();
    let lines = stdin.lock().lines().map(Result::unwrap);
    let mut boards = lines.map(|f| f.parse::<Board>().unwrap());
    (&mut boards).take(1024).collect()
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
