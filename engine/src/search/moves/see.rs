use cozy_chess::*;
use arrayvec::ArrayVec;

pub type SeeScore = i16;

const PIECE_MG_VALUE: &[SeeScore; Piece::NUM] = &[50, 298, 312, 326, 914, 20_000];
const PIECE_EG_VALUE: &[SeeScore; Piece::NUM] = &[109, 296, 317, 512, 975, 20_000];
const PIECE_VALUE_TABLE: [[SeeScore; Piece::NUM]; 25] = {
    let mut table = [[0; Piece::NUM]; 25];
    let mut phase = 0;
    while phase < 25 {
        let mut piece = 0;
        while piece < Piece::NUM {
            let mg = PIECE_MG_VALUE[piece];
            let eg = PIECE_EG_VALUE[piece];
            table[phase][piece] = ((mg as i32 * (24 - phase) as i32 + eg as i32) / 24) as SeeScore;
            piece += 1;
        }
        phase += 1;
    }
    table
};

fn phase_index(board: &Board) -> usize {
    use Piece::*;

    let c = |p| board.pieces(p).len();
    24u32.saturating_sub(c(Knight) * 1 + c(Bishop) * 1 + c(Rook) * 2 + c(Queen) * 4) as usize
}

// CITE: Static exchange evaluation.
// https://www.chessprogramming.org/Static_Exchange_Evaluation
pub fn static_exchange_evaluation(board: &Board, capture: Move) -> SeeScore {
    use Piece::*;

    let phase = phase_index(board);
    let piece_value = |p| PIECE_VALUE_TABLE[phase][p as usize];
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
            if !forced && *our_gain < 0 {
                // Choose not to make the capture.
                *our_gain = 0;
            }
        }
        return gains.pop().unwrap();
    }
}
