use cozy_chess::*;

use super::Eval;
use super::pst::{KingRelativePst, Pst, PstEvalSet};

#[derive(Debug, Clone)]
pub struct PieceEvalSet<T> {
    pub pawn: T,
    pub knight: T,
    pub bishop: T,
    pub rook: T,
    pub queen: T,
    pub king: T
}

impl<T> PieceEvalSet<T> {
    pub fn get(&self, piece: Piece) -> &T {
        match piece {
            Piece::Pawn => &self.pawn,
            Piece::Knight => &self.knight,
            Piece::Bishop => &self.bishop,
            Piece::Rook => &self.rook,
            Piece::Queen => &self.queen,
            Piece::King => &self.king
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mobility {
    pub pawn: [i16; 5],
    pub knight: [i16; 9],
    pub bishop: [i16; 14],
    pub rook: [i16; 15],
    pub queen: [i16; 28],
    pub king: [i16; 9]
}

impl Mobility {
    pub fn get(&self, piece: Piece) -> &[i16] {
        match piece {
            Piece::Pawn => &self.pawn,
            Piece::Knight => &self.knight,
            Piece::Bishop => &self.bishop,
            Piece::Rook => &self.rook,
            Piece::Queen => &self.queen,
            Piece::King => &self.king
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhasedEval {
    piece_tables: PstEvalSet,
    mobility: Mobility,
    passed_pawns: KingRelativePst,
    bishop_pair: i16,
    rook_on_open_file: i16,
    rook_on_semiopen_file: i16
}

#[derive(Debug, Clone)]
pub struct StandardEvaluator {
    pub piece_values: PieceEvalSet<i16>,
    pub midgame: PhasedEval,
    pub endgame: PhasedEval
}

impl Default for StandardEvaluator {
    fn default() -> Self {
        EVALUATOR.clone()
    }
}

impl StandardEvaluator {
    pub fn evaluate(&self, board: &Board) -> Eval {
        let phase = Self::game_phase(board);
        let us = self.evaluate_for_side(board, board.side_to_move(), phase);
        let them = self.evaluate_for_side(board, !board.side_to_move(), phase);
        Eval::cp(us - them)
    }

    pub fn piece_value(&self, piece: Piece) -> Eval {
        Eval::cp(*self.piece_values.get(piece))
    }
}

impl StandardEvaluator {
    const MAX_PHASE: u32 = 256;

    fn game_phase(board: &Board) -> u32 {
        macro_rules! game_phase_fn {
            ($($piece:ident=$weight:expr,$count:expr;)*) => {
                const INIT_PHASE: u32 = (0 $( + $count * $weight)*) * 2;
                let inv_phase = 0 $( + board.pieces(Piece::$piece).popcnt() * $weight)*;
                let phase = INIT_PHASE.saturating_sub(inv_phase); //Early promotions
                (phase * Self::MAX_PHASE + (INIT_PHASE / 2)) / INIT_PHASE
            }
        }
        game_phase_fn! {
            Pawn   = 0, 8;
            Knight = 1, 2;
            Bishop = 1, 2;
            Rook   = 2, 2;
            Queen  = 4, 1;
        }
    }

    fn evaluate_for_side(&self, board: &Board, color: Color, phase: u32) -> i16 {
        let mut value = 0;
        let mut midgame_value = 0;
        let mut endgame_value = 0;
        let our_pieces = board.colors(color);
        let their_pieces = board.colors(!color);
        let our_king = board.king(color);
        let occupied = board.occupied();

        for &piece in &Piece::ALL {
            let pieces = our_pieces & board.pieces(piece);
            let piece_value = *self.piece_values.get(piece);
            let midgame_mobility = self.midgame.mobility.get(piece);
            let endgame_mobility = self.endgame.mobility.get(piece);

            value += pieces.popcnt() as i16 * piece_value;
            for square in pieces {
                midgame_value += self.midgame.piece_tables.get(piece, color, our_king, square);
                endgame_value += self.endgame.piece_tables.get(piece, color, our_king, square);

                let approx_moves = match piece {
                    Piece::Pawn => (
                        get_pawn_quiets(square, color, occupied) |
                        (get_pawn_attacks(square, color) & board.colors(!color))
                    ),
                    Piece::Knight => get_knight_moves(square) & !our_pieces,
                    Piece::Bishop => get_bishop_moves(square, occupied) & !our_pieces,
                    Piece::Rook => get_rook_moves(square, occupied) & !our_pieces,
                    Piece::Queen => (
                        get_bishop_moves(square, occupied) |
                        get_rook_moves(square, occupied)
                    ) & !our_pieces,
                    Piece::King => get_knight_moves(square) & !our_pieces
                };
                let mobility = approx_moves.popcnt() as usize;
                midgame_value += midgame_mobility[mobility];
                endgame_value += endgame_mobility[mobility];

                if piece == Piece::Pawn {
                    let our_pawns = pieces;
                    let their_pawns = their_pieces & board.pieces(Piece::Pawn);

                    let promotion_rank = Rank::Eighth.relative_to(color);
                    let telestop = Square::new(square.file(), promotion_rank);
                    let front_span = get_between_rays(square, telestop);
                    let mut blocker_mask = front_span;
                    for attack in get_pawn_attacks(square, color) {
                        let telestop = Square::new(attack.file(), promotion_rank);
                        let front_span = get_between_rays(attack, telestop);
                        blocker_mask |= front_span | attack.bitboard();
                    }

                    let passed = (their_pawns & blocker_mask).is_empty()
                        && (our_pawns & front_span).is_empty();
                    if passed {
                        midgame_value += self.midgame.passed_pawns.get(color, our_king, square);
                        endgame_value += self.endgame.passed_pawns.get(color, our_king, square);
                    }
                }

                if piece == Piece::Rook {
                    let pawns = board.pieces(Piece::Pawn);
                    let our_pawns = our_pieces & pawns;
                    let file = square.file();
                    let file_bb = file.bitboard();
                    if (file_bb & pawns).is_empty() {
                        midgame_value += self.midgame.rook_on_open_file;
                        endgame_value += self.endgame.rook_on_open_file;
                    } else if (file_bb & our_pawns).is_empty() {
                        midgame_value += self.midgame.rook_on_semiopen_file;
                        endgame_value += self.endgame.rook_on_semiopen_file;
                    }
                }
            }
        }
        if (our_pieces & board.pieces(Piece::Bishop)).popcnt() >= 2 {
            midgame_value += self.midgame.bishop_pair;
            endgame_value += self.endgame.bishop_pair;
        }

        midgame_value += value;
        endgame_value += value;
        let phase = phase as i32;
        const MAX_PHASE: i32 = StandardEvaluator::MAX_PHASE as i32;
        let interpolated = (
            (midgame_value as i32 * (MAX_PHASE - phase)) +
            (endgame_value as i32 * phase)
        ) / MAX_PHASE;
        interpolated as i16
    }
}

pub const EVALUATOR: StandardEvaluator = StandardEvaluator {
    piece_values: PieceEvalSet {
        pawn: 100,
        knight: 320,
        bishop: 330,
        rook: 500,
        queen: 900,
        king: 0,
    },
    midgame: PhasedEval {
        piece_tables: PstEvalSet {
            pawn: KingRelativePst([
                [
                    [   0,    0,    0,    0],
                    [  -3,   13,   25,   37],
                    [ -38,  -14,   12,   15],
                    [ -46,  -19,  -14,   -5],
                    [ -52,  -20,  -16,   -2],
                    [ -51,  -25,  -21,  -13],
                    [ -60,  -35,  -34,  -34],
                    [   0,    0,    0,    0],
                ],
                [
                    [   0,    0,    0,    0],
                    [ -15,   -2,   24,   37],
                    [  26,   59,   54,   25],
                    [   9,   22,   15,   15],
                    [   0,   18,   10,    7],
                    [  12,   37,   11,    0],
                    [ -11,   38,    8,  -19],
                    [   0,    0,    0,    0],
                ],
            ]),
            knight: KingRelativePst([
                [
                    [-141,  -60,  -50,  -25],
                    [ -12,   -1,   30,   49],
                    [ -11,   28,   56,   63],
                    [  -2,   11,   37,   54],
                    [  -7,   13,   24,   25],
                    [ -15,    5,   13,   20],
                    [ -36,  -21,   -8,    5],
                    [ -70,  -18,  -29,  -14],
                ],
                [
                    [-107,  -35,  -37,   14],
                    [   8,   12,   80,   30],
                    [  20,   46,   90,   94],
                    [  23,   20,   63,   41],
                    [   9,   32,   35,   34],
                    [   5,   29,   34,   33],
                    [ -13,  -15,    5,    6],
                    [ -56,  -15,  -11,   -7],
                ],
            ]),
            bishop: KingRelativePst([
                [
                    [ -19,  -30,  -52,  -67],
                    [  -8,   19,    8,  -18],
                    [   0,   17,   20,   35],
                    [  -7,    3,   23,   35],
                    [   5,    6,    8,   27],
                    [  13,   26,   18,   14],
                    [  14,   16,   21,    5],
                    [   2,   24,   10,    1],
                ],
                [
                    [ -33,  -20,  -47,  -52],
                    [ -15,   -6,    8,   14],
                    [  13,   33,   46,   29],
                    [  -8,    2,   23,   27],
                    [  19,   11,   15,   27],
                    [  36,   35,   34,   19],
                    [  10,   34,   26,   15],
                    [  10,   15,   -3,    8],
                ],
            ]),
            rook: KingRelativePst([
                [
                    [   7,    9,    2,   -2],
                    [  -4,   -7,   11,   31],
                    [ -25,   -4,  -11,   -3],
                    [ -33,  -23,  -21,  -14],
                    [ -42,  -40,  -33,  -27],
                    [ -37,  -28,  -26,  -24],
                    [ -37,  -29,  -13,  -13],
                    [ -12,  -12,   -7,    0],
                ],
                [
                    [  47,    0,   20,   22],
                    [  65,   47,   52,   17],
                    [  37,   60,   29,   21],
                    [   4,   -3,    0,  -15],
                    [ -12,    6,  -18,  -20],
                    [  12,   30,   -1,  -11],
                    [ -14,   16,   -3,   -7],
                    [ -10,   -4,   -2,    5],
                ],
            ]),
            queen: KingRelativePst([
                [
                    [ -39,  -32,  -12,   15],
                    [  -6,  -22,  -16,  -33],
                    [  -1,   -5,  -10,   -5],
                    [ -14,  -15,  -11,  -15],
                    [  -4,   -8,  -11,  -10],
                    [   8,    7,    1,   -3],
                    [   3,    0,    6,    9],
                    [  -7,   -4,    5,    9],
                ],
                [
                    [   5,   41,   20,    6],
                    [  32,  -11,   10,  -27],
                    [  26,   38,   14,    9],
                    [   1,   -7,   -6,  -15],
                    [  14,    9,    0,   -4],
                    [  22,   25,   20,   -1],
                    [  -1,   16,   15,    9],
                    [ -12,  -17,  -13,    8],
                ],
            ]),
            king: Pst([
                [ -10,    6,   12,   -8,   -3,    5,    9,   -2],
                [ -19,   13,    1,   37,   14,   18,   27,    8],
                [ -27,   45,   13,   -2,    9,   53,   40,    9],
                [ -25,  -13,  -18,  -63,  -74,  -33,  -29,  -68],
                [ -35,  -14,  -47,  -94,  -91,  -49,  -43,  -93],
                [   1,   35,  -21,  -45,  -40,  -24,   18,  -18],
                [  61,   60,   29,    7,    3,   21,   68,   54],
                [  52,   77,   50,  -24,   38,    4,   71,   65],
            ]),
        },
        mobility: Mobility {
            pawn: [-17, -9, 1, 29, 0],
            knight: [-63, -48, -40, -37, -34, -36, -36, -36, -34],
            bishop: [-65, -58, -51, -48, -42, -35, -30, -27, -25, -24, -21, -18, -12, 4],
            rook: [-130, -119, -115, -110, -110, -105, -101, -96, -94, -91, -88, -83, -82, -72, -74],
            queen: [-38, -44, -53, -52, -50, -47, -44, -46, -43, -42, -41, -41, -40, -39, -38, -38, -35, -38, -36, -34, -23, -21, -16, -13, 12, 31, 3, 9],
            king: [2, 6, -2, -11, -21, -38, -42, -56, -55],
        },
        passed_pawns: KingRelativePst([
            [
                [   0,    0,    0,    0],
                [   4,   25,   25,   33],
                [  22,   41,   32,   16],
                [  29,   22,   27,   14],
                [  17,    2,  -11,   -8],
                [   5,  -14,  -17,  -18],
                [   1,   -4,   -9,  -13],
                [   0,    0,    0,    0],
            ],
            [
                [   0,    0,    0,    0],
                [ -35,  -11,   15,   32],
                [ -66,  -41,    3,   13],
                [ -40,  -18,    7,   -3],
                [ -37,  -15,  -17,  -16],
                [ -15,   -3,  -27,  -18],
                [ -23,   10,   -3,   -3],
                [   0,    0,    0,    0],
            ],
        ]),
        bishop_pair: 20,
        rook_on_open_file: 25,
        rook_on_semiopen_file: 9,
    },
    endgame: PhasedEval {
        piece_tables: PstEvalSet {
            pawn: KingRelativePst([
                [
                    [   0,    0,    0,    0],
                    [  91,   80,   72,   64],
                    [   1,    7,  -16,    2],
                    [  -3,   -4,  -16,  -23],
                    [ -15,   -8,  -18,  -20],
                    [ -18,  -10,  -20,  -13],
                    [ -17,  -12,  -19,  -11],
                    [   0,    0,    0,    0],
                ],
                [
                    [   0,    0,    0,    0],
                    [ 122,  106,   98,   70],
                    [  -8,    4,  -27,  -23],
                    [ -20,  -12,  -25,  -27],
                    [ -29,  -15,  -24,  -21],
                    [ -35,  -20,  -19,  -13],
                    [ -41,  -27,  -19,   -2],
                    [   0,    0,    0,    0],
                ],
            ]),
            knight: KingRelativePst([
                [
                    [ -72,  -24,  -10,   -8],
                    [ -17,    1,   -1,   -7],
                    [ -10,   -5,   15,   19],
                    [   0,    9,   27,   31],
                    [   4,    7,   28,   27],
                    [  -7,    3,   14,   20],
                    [ -23,  -16,   -2,   -4],
                    [ -38,  -32,  -19,  -14],
                ],
                [
                    [-104,  -54,  -33,  -21],
                    [ -36,  -16,  -25,   -8],
                    [ -28,  -15,   -7,    5],
                    [ -13,    7,   19,   26],
                    [  -3,    5,   20,   28],
                    [  -7,    4,   13,   16],
                    [ -13,  -10,   -6,   -1],
                    [ -25,  -26,  -14,  -11],
                ],
            ]),
            bishop: KingRelativePst([
                [
                    [  -6,   -6,   -8,    0],
                    [ -19,  -17,  -16,   -8],
                    [ -12,  -15,  -10,  -22],
                    [ -11,   -6,  -12,    1],
                    [ -12,   -6,    1,   -5],
                    [  -6,   -1,   -2,   -6],
                    [ -20,  -20,  -27,  -11],
                    [ -27,  -13,  -17,  -15],
                ],
                [
                    [ -24,   -9,  -10,   -7],
                    [ -24,   -9,  -18,  -24],
                    [ -13,  -17,  -14,  -12],
                    [ -14,  -14,   -8,   -4],
                    [ -20,   -6,   -6,   -4],
                    [  -8,   -4,   -2,   -5],
                    [ -16,  -15,  -18,  -14],
                    [ -34,  -20,    1,  -11],
                ],
            ]),
            rook: KingRelativePst([
                [
                    [  14,   11,   21,   21],
                    [  14,   19,   21,   13],
                    [  11,    7,   11,    6],
                    [  14,    7,   11,    7],
                    [  13,   10,    9,    6],
                    [   4,   -1,    0,    2],
                    [  -2,   -1,   -4,   -5],
                    [  -1,   -6,   -2,   -7],
                ],
                [
                    [  -3,   17,   16,    8],
                    [ -18,    0,    2,   12],
                    [ -20,  -16,  -14,   -4],
                    [  -9,   -4,   -5,   -1],
                    [  -5,   -5,    0,    2],
                    [ -17,  -19,   -9,   -7],
                    [ -16,  -24,  -11,   -8],
                    [ -17,   -4,    0,   -9],
                ],
            ]),
            queen: KingRelativePst([
                [
                    [  31,   24,   36,   26],
                    [   5,   15,   35,   72],
                    [  -9,   -7,   25,   47],
                    [   3,    7,   18,   41],
                    [   3,    9,   13,   30],
                    [ -13,   -1,    9,    3],
                    [ -25,  -10,  -15,  -14],
                    [ -14,  -15,  -24,   -5],
                ],
                [
                    [  -5,  -10,   38,   42],
                    [  15,   38,   54,   81],
                    [  18,    7,   48,   51],
                    [  35,   43,   53,   53],
                    [  25,   36,   32,   34],
                    [   3,   22,   18,   13],
                    [ -33,  -48,  -27,   -9],
                    [ -34,  -31,  -10,  -22],
                ],
            ]),
            king: Pst([
                [ -83,  -47,  -30,   -8,  -24,  -16,  -15,  -81],
                [ -23,   16,   20,   21,   33,   41,   29,  -10],
                [ -10,   18,   35,   47,   50,   42,   32,   -3],
                [ -13,   19,   39,   54,   56,   45,   29,    2],
                [ -21,    9,   34,   52,   52,   34,   19,    5],
                [ -28,   -2,   19,   33,   30,   22,    2,   -7],
                [ -46,  -23,   -3,    5,    8,   -1,  -22,  -37],
                [ -75,  -60,  -39,  -24,  -45,  -26,  -53,  -75],
            ]),
        },
        mobility: Mobility {
            pawn: [3, 24, 30, 22, 50],
            knight: [34, 31, 37, 33, 38, 42, 43, 42, 37],
            bishop: [-15, 1, 9, 20, 33, 45, 49, 56, 63, 62, 61, 61, 65, 54],
            rook: [105, 123, 126, 129, 135, 137, 139, 141, 147, 151, 152, 154, 158, 160, 158],
            queen: [7, 45, 151, 186, 196, 201, 210, 234, 241, 247, 256, 262, 271, 278, 284, 290, 293, 304, 311, 311, 310, 313, 311, 317, 296, 301, 301, 303],
            king: [-1, -23, -18, -4, 0, 4, 4, 4, -3],
        },
        passed_pawns: KingRelativePst([
            [
                [   0,    0,    0,    0],
                [  98,   92,   73,   61],
                [ 135,  119,  101,   65],
                [  63,   52,   35,   37],
                [  32,   18,   15,   11],
                [   3,   11,    6,   -6],
                [   2,    6,    1,  -15],
                [   0,    0,    0,    0],
            ],
            [
                [   0,    0,    0,    0],
                [ 102,   98,   88,   65],
                [ 170,  149,  133,   94],
                [  93,   84,   68,   46],
                [  54,   49,   35,   24],
                [  12,   14,   11,    6],
                [  11,    2,   -1,   -8],
                [   0,    0,    0,    0],
            ],
        ]),
        bishop_pair: 64,
        rook_on_open_file: 4,
        rook_on_semiopen_file: 2,
    },
};
