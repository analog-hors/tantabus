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
        Piece::King => 0,
    })
}

fn get_both_pawn_attacks(sq: Square) -> BitBoard {
    get_pawn_attacks(sq, Color::White) | get_pawn_attacks(sq, Color::Black)
}

// CITE: Static exchange evaluation.
// https://www.chessprogramming.org/Static_Exchange_Evaluation
pub fn static_exchange_evaluation(board: &Board, capture: Move) -> Eval {
    use Piece::*;

    let sq = capture.to;
    let mut attacker_sq = capture.from;
    let mut victim = board.piece_on(sq).unwrap();
    let mut attacker = board.piece_on(attacker_sq).unwrap();
    let mut color = board.side_to_move();
    let mut blockers = board.occupied();
    let mut attackers =
        get_king_moves(sq)                   & board.pieces(King) |
        get_knight_moves(sq)                 & board.pieces(Knight) |
        get_rook_moves(sq, blockers)         & (board.pieces(Rook) | board.pieces(Queen)) |
        get_bishop_moves(sq, blockers)       & (board.pieces(Bishop) | board.pieces(Queen)) |
        get_both_pawn_attacks(sq) & blockers & board.pieces(Pawn);

    // 32 pieces max on a legal chess board.
    let mut captures = ArrayVec::<_, 32>::new();
    'exchange: loop {
        // "Capture" victim
        captures.push(piece_value(victim));

        // "Move" attacker to target square
        blockers ^= attacker_sq.bitboard();
        attackers ^= attacker_sq.bitboard();

        // Add new exposed sliding pieces
        if matches!(attacker, Rook | Queen) {
            attackers |= get_rook_moves(sq, blockers)
                & blockers
                & (board.pieces(Rook) | board.pieces(Queen));
        }
        if matches!(attacker, Pawn | Bishop | Queen) {
            attackers |= get_bishop_moves(sq, blockers)
                & blockers
                & (board.pieces(Bishop) | board.pieces(Queen));
        }

        // Swap sides
        color = !color;

        // Try to fetch a new attacker
        for &new_attacker in &Piece::ALL {
            let attackers = attackers & board.colored_pieces(color, new_attacker);
            if let Some(sq) = attackers.next_square() {
                if victim == Piece::King {
                    // Oops! Our last capture with our king was illegal since this piece is defended.
                    captures.pop();
                    break;
                }

                // New attacker, the old attacker is now the victim
                victim = attacker;
                attacker = new_attacker;
                attacker_sq = sq;
                continue 'exchange;
            }
        }

        // No attacker could be found, calculate final result.
        while captures.len() > 1 {
            // First capture is forced, but all others can be ignored.
            let forced = captures.len() == 2;
            let their_gain = captures.pop().unwrap();
            let our_gain = captures.last_mut().unwrap();
            *our_gain -= their_gain;
            if !forced && *our_gain < Eval::ZERO {
                // Choose not to make the capture.
                *our_gain = Eval::ZERO;
            }
        }
        return captures.pop().unwrap();
    }
}
