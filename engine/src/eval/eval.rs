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
    bishop_pair: i16
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
                    [ -39,  -14,   12,   15],
                    [ -49,  -17,  -14,   -4],
                    [ -54,  -20,  -16,    2],
                    [ -50,  -24,  -19,  -10],
                    [ -62,  -32,  -34,  -34],
                    [   0,    0,    0,    0],
                ],
                [
                    [   0,    0,    0,    0],
                    [ -15,   -2,   24,   37],
                    [  25,   59,   54,   24],
                    [   8,   23,   12,   16],
                    [  -1,   17,    8,    9],
                    [  15,   37,    9,    1],
                    [ -10,   37,    6,  -18],
                    [   0,    0,    0,    0],
                ],
            ]),
            knight: KingRelativePst([
                [
                    [-141,  -60,  -50,  -25],
                    [ -12,   -1,   30,   49],
                    [ -11,   28,   56,   62],
                    [  -2,   12,   37,   54],
                    [  -7,   13,   24,   25],
                    [ -16,    5,   14,   20],
                    [ -36,  -21,   -9,    7],
                    [ -70,  -17,  -30,  -14],
                ],
                [
                    [-107,  -35,  -37,   14],
                    [   8,   12,   80,   30],
                    [  20,   46,   90,   94],
                    [  23,   20,   63,   42],
                    [   8,   31,   35,   35],
                    [   4,   28,   36,   32],
                    [ -13,  -15,    5,    5],
                    [ -56,  -16,  -11,   -8],
                ],
            ]),
            bishop: KingRelativePst([
                [
                    [ -18,  -30,  -52,  -67],
                    [  -8,   19,    8,  -18],
                    [   0,   17,   21,   35],
                    [  -7,    2,   23,   35],
                    [   5,    6,    9,   28],
                    [  13,   27,   17,   16],
                    [  14,   16,   21,    3],
                    [   2,   24,   11,    0],
                ],
                [
                    [ -33,  -20,  -46,  -52],
                    [ -15,   -6,    8,   14],
                    [  14,   33,   47,   30],
                    [  -8,    3,   23,   28],
                    [  19,   11,   16,   27],
                    [  36,   36,   35,   19],
                    [  11,   36,   27,   15],
                    [  10,   15,   -6,    9],
                ],
            ]),
            rook: KingRelativePst([
                [
                    [   8,    9,    3,   -2],
                    [  -2,   -6,   13,   32],
                    [ -24,   -3,   -9,   -2],
                    [ -32,  -23,  -20,  -13],
                    [ -42,  -39,  -32,  -26],
                    [ -37,  -28,  -25,  -24],
                    [ -37,  -29,  -12,  -10],
                    [ -14,  -11,   -3,    5],
                ],
                [
                    [  47,    0,   20,   22],
                    [  66,   47,   53,   18],
                    [  38,   60,   30,   22],
                    [   4,   -3,    0,  -14],
                    [ -12,    6,  -18,  -19],
                    [  12,   29,   -2,  -11],
                    [ -15,   15,   -3,   -6],
                    [ -13,   -2,   -3,    8],
                ],
            ]),
            queen: KingRelativePst([
                [
                    [ -39,  -33,  -13,   14],
                    [  -6,  -21,  -16,  -33],
                    [   0,   -5,  -10,   -5],
                    [ -14,  -15,  -11,  -15],
                    [  -4,   -8,  -11,  -10],
                    [   8,    8,    0,   -3],
                    [   4,    1,    6,    8],
                    [  -8,   -4,    5,   11],
                ],
                [
                    [   5,   41,   20,    6],
                    [  32,  -11,   10,  -27],
                    [  26,   39,   15,    9],
                    [   0,   -8,   -6,  -15],
                    [  13,    8,   -1,   -5],
                    [  22,   25,   19,   -1],
                    [  -1,   16,   16,   11],
                    [ -12,  -17,  -13,    7],
                ],
            ]),
            king: Pst([
                [ -10,    6,   12,   -7,   -3,    5,    9,   -2],
                [ -19,   13,    1,   37,   14,   18,   27,    8],
                [ -27,   45,   13,   -2,    9,   53,   40,    9],
                [ -25,  -13,  -18,  -63,  -74,  -33,  -29,  -68],
                [ -35,  -14,  -47,  -94,  -91,  -49,  -43,  -94],
                [   0,   35,  -21,  -45,  -40,  -24,   18,  -18],
                [  60,   59,   29,    7,    3,   22,   65,   51],
                [  52,   77,   51,  -24,   39,    3,   72,   67],
            ]),
        },
        mobility: Mobility {
            pawn: [-16, -8, 0, 28, 0],
            knight: [-64, -49, -41, -39, -35, -36, -36, -36, -35],
            bishop: [-69, -61, -52, -49, -42, -32, -27, -24, -24, -23, -21, -18, -12, 4],
            rook: [-132, -123, -117, -113, -113, -107, -102, -93, -89, -86, -81, -76, -75, -69, -70],
            queen: [-38, -45, -54, -52, -50, -48, -42, -46, -42, -41, -40, -40, -41, -39, -38, -39, -35, -39, -37, -35, -24, -22, -17, -13, 12, 31, 3, 9],
            king: [2, 8, -1, -12, -23, -39, -43, -54, -54],
        },
        passed_pawns: KingRelativePst([
            [
                [   0,    0,    0,    0],
                [   4,   25,   25,   33],
                [  22,   41,   32,   15],
                [  29,   23,   27,   14],
                [  17,    2,  -10,   -6],
                [   5,  -14,  -17,  -18],
                [   0,   -4,   -9,  -13],
                [   0,    0,    0,    0],
            ],
            [
                [   0,    0,    0,    0],
                [ -35,  -11,   15,   32],
                [ -66,  -41,    3,   12],
                [ -40,  -18,    7,   -2],
                [ -37,  -15,  -17,  -16],
                [ -16,   -3,  -27,  -19],
                [ -24,    9,   -3,   -3],
                [   0,    0,    0,    0],
            ],
        ]),
        bishop_pair: 24,
    },
    endgame: PhasedEval {
        piece_tables: PstEvalSet {
            pawn: KingRelativePst([
                [
                    [   0,    0,    0,    0],
                    [  91,   80,   72,   64],
                    [   0,    7,  -16,    1],
                    [  -4,   -2,  -14,  -21],
                    [ -16,   -6,  -17,  -16],
                    [ -16,   -8,  -19,  -14],
                    [ -16,   -9,  -18,  -11],
                    [   0,    0,    0,    0],
                ],
                [
                    [   0,    0,    0,    0],
                    [ 122,  106,   98,   70],
                    [  -8,    4,  -27,  -24],
                    [ -21,  -12,  -26,  -26],
                    [ -29,  -16,  -23,  -18],
                    [ -34,  -20,  -21,  -12],
                    [ -40,  -29,  -20,   -1],
                    [   0,    0,    0,    0],
                ],
            ]),
            knight: KingRelativePst([
                [
                    [ -72,  -24,  -10,   -8],
                    [ -17,    1,   -1,   -7],
                    [ -10,   -4,   15,   19],
                    [   0,    9,   28,   31],
                    [   3,    7,   28,   27],
                    [  -8,    3,   15,   20],
                    [ -23,  -16,   -2,   -3],
                    [ -38,  -33,  -19,  -15],
                ],
                [
                    [-104,  -54,  -33,  -21],
                    [ -36,  -16,  -25,   -8],
                    [ -28,  -15,   -6,    6],
                    [ -13,    7,   19,   27],
                    [  -4,    4,   20,   28],
                    [  -7,    4,   14,   16],
                    [ -13,  -10,   -6,   -2],
                    [ -25,  -27,  -14,  -12],
                ],
            ]),
            bishop: KingRelativePst([
                [
                    [  -5,   -6,   -8,    1],
                    [ -19,  -17,  -16,   -7],
                    [ -11,  -15,   -9,  -21],
                    [ -10,   -6,  -11,    2],
                    [ -12,   -6,    2,   -5],
                    [  -6,    0,   -1,   -5],
                    [ -20,  -19,  -26,  -11],
                    [ -27,  -14,  -17,  -15],
                ],
                [
                    [ -24,   -9,  -10,   -7],
                    [ -24,   -9,  -17,  -24],
                    [ -13,  -16,  -13,  -11],
                    [ -14,  -13,   -7,   -3],
                    [ -20,   -6,   -6,   -4],
                    [  -8,   -3,   -2,   -4],
                    [ -16,  -15,  -18,  -14],
                    [ -34,  -20,   -1,  -12],
                ],
            ]),
            rook: KingRelativePst([
                [
                    [  14,   11,   21,   22],
                    [  15,   21,   23,   15],
                    [  12,    8,   13,    7],
                    [  14,    8,   12,    8],
                    [  12,   10,   10,    7],
                    [   4,   -1,    1,    3],
                    [  -2,   -2,   -3,   -3],
                    [  -1,   -6,   -2,   -5],
                ],
                [
                    [  -3,   17,   16,    9],
                    [ -17,    0,    3,   14],
                    [ -20,  -16,  -13,   -3],
                    [  -9,   -4,   -5,    0],
                    [  -5,   -5,    0,    4],
                    [ -17,  -19,   -9,   -6],
                    [ -16,  -24,  -10,   -7],
                    [ -18,   -4,    0,   -7],
                ],
            ]),
            queen: KingRelativePst([
                [
                    [  30,   24,   35,   26],
                    [   5,   16,   35,   72],
                    [  -9,   -6,   26,   47],
                    [   4,    7,   19,   42],
                    [   4,    9,   14,   31],
                    [ -13,    0,    9,    3],
                    [ -25,  -10,  -15,  -14],
                    [ -15,  -15,  -24,   -5],
                ],
                [
                    [  -5,   -9,   37,   42],
                    [  15,   38,   54,   81],
                    [  18,    7,   48,   51],
                    [  35,   43,   53,   53],
                    [  24,   36,   31,   34],
                    [   3,   22,   18,   13],
                    [ -33,  -48,  -27,   -8],
                    [ -34,  -31,  -10,  -23],
                ],
            ]),
            king: Pst([
                [ -83,  -47,  -30,   -8,  -24,  -16,  -15,  -80],
                [ -23,   16,   20,   21,   33,   41,   29,  -10],
                [ -10,   19,   35,   47,   50,   42,   32,   -3],
                [ -13,   19,   39,   54,   56,   45,   29,    2],
                [ -21,    9,   35,   52,   52,   34,   19,    5],
                [ -28,   -2,   20,   33,   31,   21,    2,   -7],
                [ -46,  -23,   -3,    5,    8,   -1,  -25,  -40],
                [ -75,  -60,  -38,  -24,  -44,  -26,  -53,  -75],
            ]),
        },
        mobility: Mobility {
            pawn: [7, 31, 35, 22, 50],
            knight: [34, 30, 38, 33, 37, 43, 43, 42, 37],
            bishop: [-16, 1, 9, 19, 32, 46, 49, 57, 63, 63, 61, 61, 65, 53],
            rook: [105, 123, 126, 129, 135, 137, 138, 141, 148, 154, 155, 158, 163, 164, 164],
            queen: [7, 45, 151, 186, 196, 201, 211, 234, 241, 247, 256, 263, 271, 278, 284, 290, 293, 304, 311, 311, 310, 313, 311, 317, 296, 301, 301, 303],
            king: [-1, -22, -17, -5, -2, 3, 3, 7, -1],
        },
        passed_pawns: KingRelativePst([
            [
                [   0,    0,    0,    0],
                [  98,   92,   73,   60],
                [ 135,  119,  101,   64],
                [  64,   53,   36,   37],
                [  33,   19,   16,   12],
                [   3,   11,    7,   -6],
                [   1,    7,    1,  -15],
                [   0,    0,    0,    0],
            ],
            [
                [   0,    0,    0,    0],
                [ 101,   98,   88,   65],
                [ 170,  149,  133,   93],
                [  94,   85,   68,   46],
                [  55,   49,   35,   25],
                [  12,   14,   11,    6],
                [  11,    1,   -2,   -8],
                [   0,    0,    0,    0],
            ],
        ]),
        bishop_pair: 64,
    },
}
;
