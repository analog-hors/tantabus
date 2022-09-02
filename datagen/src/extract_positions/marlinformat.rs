use std::io::Write;

use cozy_chess::*;

const UNMOVED_ROOK: u8 = Piece::NUM as u8;
const NO_SQUARE: u8 = Square::NUM as u8;

fn square_index(bitboard: BitBoard, square: Square) -> usize {
    let squares_behind = BitBoard(square.bitboard().0 - 1);
    (bitboard & squares_behind).len() as usize
}

pub fn write_as_marlinformat(out: &mut impl Write, board: &Board, cp: i16, winner: Option<Color>) -> std::io::Result<()> {
    out.write_all(&board.occupied().0.to_le_bytes())?;

    let mut unmoved_rooks = BitBoard::EMPTY;
    let castling_rights = board.castle_rights(board.side_to_move());
    let back_rank = Rank::First.relative_to(board.side_to_move());
    if let Some(file) = castling_rights.short {
        unmoved_rooks |= Square::new(file, back_rank).bitboard();
    }
    if let Some(file) = castling_rights.long {
        unmoved_rooks |= Square::new(file, back_rank).bitboard();
    }
    let mut encoded_pieces = [0; 32];
    for &color in &Color::ALL {
        for &piece in &Piece::ALL {
            for square in board.colors(color) & board.pieces(piece) {
                let encoded_piece = if unmoved_rooks.has(square) {
                    UNMOVED_ROOK
                } else {
                    piece as u8
                };
                let index = square_index(board.occupied(), square);
                encoded_pieces[index] = encoded_piece | (color as u8) << 3;
            }
        }
    }
    for piece_pair in encoded_pieces.chunks_exact(2) {
        out.write_all(&[piece_pair[1] << 4 | piece_pair[0]])?;
    }

    let encoded_ep_square = board.en_passant().map_or(NO_SQUARE, |f| {
        Square::new(f, Rank::Sixth.relative_to(board.side_to_move())) as u8
    });
    out.write_all(&[(board.side_to_move() as u8) << 7 | encoded_ep_square])?;

    out.write_all(&[board.halfmove_clock()])?;
    out.write_all(&board.fullmove_number().to_le_bytes())?;
    

    let wdl = match winner {
        Some(Color::White) => 2,
        Some(Color::Black) => 0,
        None => 1,
    };
    out.write_all(&cp.to_le_bytes())?;
    out.write_all(&[wdl])?;
    out.write_all(&[0])?;

    Ok(())
}
