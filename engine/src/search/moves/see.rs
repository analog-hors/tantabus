use cozy_chess::*;
use arrayvec::ArrayVec;

use crate::eval::*;

fn piece_value(piece: Piece) -> Eval {
    Eval::cp(match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        // King capture is legal in SEE's simulation of real chess.
        Piece::King => 20_000,
    })
}

// CITE: Static exchange evaluation.
// https://www.chessprogramming.org/Static_Exchange_Evaluation
pub fn static_exchange_evaluation(board: &Board, capture: Move) -> Eval {
    use Piece::*;

    let target_sq = capture.to;
    let initial_capture = board.piece_on(target_sq).unwrap();
    let initial_color = board.side_to_move();

    // Attacker moved to target square, so remove it
    let mut blockers = board.occupied() ^ capture.from.bitboard();
    let mut attackers =
        get_king_moves(target_sq) & blockers                 & board.pieces(King) |
        get_knight_moves(target_sq) & blockers               & board.pieces(Knight) |
        get_rook_moves(target_sq, blockers) & blockers       & (board.pieces(Rook) | board.pieces(Queen)) |
        get_bishop_moves(target_sq, blockers) & blockers     & (board.pieces(Bishop) | board.pieces(Queen)) |
        get_pawn_attacks(target_sq, Color::Black) & blockers & board.colored_pieces(Color::White, Pawn) |
        get_pawn_attacks(target_sq, Color::White) & blockers & board.colored_pieces(Color::Black, Pawn);

    // Attacker moved to the target square
    let mut target_piece = board.piece_on(capture.from).unwrap();
    let mut color = !initial_color;

    let mut gains = ArrayVec::<_, 32>::new();
    gains.push(piece_value(initial_capture));

    'exchange: loop {
        // Find least valuable piece to capture victim
        for &attacker_piece in &Piece::ALL {
            let our_attackers = attackers & board.colored_pieces(color, attacker_piece);
            if let Some(attacker_sq) = our_attackers.next_square() {                
                // "Capture" victim
                let victim_value = piece_value(target_piece);
                gains.push(victim_value);

                // We captured the king lol
                if target_piece == Piece::King {
                    break;
                }

                // "Move" attacker to target square
                blockers ^= attacker_sq.bitboard();
                attackers ^= attacker_sq.bitboard();
                target_piece = attacker_piece;

                // Add new exposed sliding pieces
                if matches!(attacker_piece, Rook | Queen) {
                    attackers |= get_rook_moves(target_sq, blockers)
                        & blockers
                        & (board.pieces(Rook) | board.pieces(Queen));
                }
                if matches!(attacker_piece, Pawn | Bishop | Queen) {
                    attackers |= get_bishop_moves(target_sq, blockers)
                        & blockers
                        & (board.pieces(Bishop) | board.pieces(Queen));
                }

                // Swap sides
                color = !color;
                
                continue 'exchange;
            }
        }

        // No attacker could be found, calculate final result.
        while gains.len() > 1 {
            // First capture is forced, but all others can be ignored.
            let forced = gains.len() == 2;
            let their_gain = gains.pop().unwrap();
            let our_gain = gains.last_mut().unwrap();
            *our_gain -= their_gain;
            if !forced && *our_gain < Eval::ZERO {
                // Choose not to make the capture.
                *our_gain = Eval::ZERO;
            }
        }
        return gains.pop().unwrap();
    }
}
