use cozy_chess::*;

use super::Eval;

#[derive(Debug, Clone)]
pub struct PieceSquareTable(pub [[i16; 8]; 8]);

impl PieceSquareTable {
    fn key(side: Color, square: Square) -> (usize, usize) {
        let rank = square.rank().relative_to(!side);
        (rank as usize, square.file() as usize)
    }
    
    pub fn get(&self, side: Color, square: Square) -> i16 {
        let (rank, file) = Self::key(side, square);
        self.0[rank][file]
    }

    pub fn set(&mut self, side: Color, square: Square, value: i16) {
        let (rank, file) = Self::key(side, square);
        self.0[rank][file] = value;
    }
}

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
    piece_tables: PieceEvalSet<PieceSquareTable>,
    mobility: Mobility,
    passed_pawns: PieceSquareTable,
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
        let occupied = board.occupied();

        for &piece in &Piece::ALL {
            let pieces = our_pieces & board.pieces(piece);
            let piece_value = *self.piece_values.get(piece);
            let midgame_piece_table = self.midgame.piece_tables.get(piece);
            let endgame_piece_table = self.endgame.piece_tables.get(piece);
            let midgame_mobility = self.midgame.mobility.get(piece);
            let endgame_mobility = self.endgame.mobility.get(piece);

            value += pieces.popcnt() as i16 * piece_value;
            for square in pieces {
                midgame_value += midgame_piece_table.get(color, square);
                endgame_value += endgame_piece_table.get(color, square);

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
                        midgame_value += self.midgame.passed_pawns.get(color, square);
                        endgame_value += self.endgame.passed_pawns.get(color, square);
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
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 112,  110,  109,  117,  112,   74,  -20,  -17],
                [ -60,  -22,  -17,  -14,    8,   42,    3,  -37],
                [ -52,  -32,  -35,  -23,  -13,  -10,  -16,  -42],
                [ -52,  -35,  -35,  -34,  -23,  -28,  -17,  -38],
                [ -52,  -41,  -42,  -38,  -31,  -29,  -13,  -31],
                [ -58,  -49,  -50,  -51,  -53,  -29,  -12,  -46],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-178,  -57, -106,  -18,    1,  -65,  -14, -108],
                [ -33,  -26,   15,   40,   29,   67,  -28,    9],
                [ -50,   -8,   26,   37,   68,   69,   21,    4],
                [ -19,   -7,   16,   19,    7,   31,    3,    5],
                [ -23,   -9,   -1,   -2,    6,    0,   17,  -12],
                [ -29,  -15,  -10,   -1,    2,    1,   -7,  -20],
                [ -49,  -50,  -28,  -19,  -22,  -20,  -37,  -33],
                [-108,  -30,  -58,  -34,  -29,  -32,  -29, -108],
            ]),
            bishop: PieceSquareTable([
                [ -77,  -71, -103,  -98,  -69, -104,   -4,  -59],
                [ -60,  -29,  -37,  -54,  -36,  -12,  -42,  -19],
                [ -28,  -18,   -1,   -2,   22,   27,   15,  -12],
                [ -35,  -13,  -13,   13,   -7,    4,  -18,  -26],
                [ -30,  -15,  -16,   -8,   -4,  -21,  -14,  -15],
                [ -22,   -8,  -15,  -14,  -17,  -10,   -8,   -7],
                [ -17,  -14,  -13,  -26,  -20,  -17,   -6,  -18],
                [ -13,  -17,  -28,  -36,  -36,  -28,  -29,  -21],
            ]),
            rook: PieceSquareTable([
                [ -40,  -44,  -50,  -45,  -38,  -25,   -6,   13],
                [ -75,  -81,  -56,  -38,  -39,    1,  -47,  -11],
                [-101,  -65,  -71,  -50,  -25,  -25,    0,  -44],
                [-104,  -95,  -88,  -71,  -79,  -63,  -64,  -65],
                [-112, -112, -109, -103, -103,  -96,  -73,  -93],
                [-115, -105, -110,  -98,  -98,  -96,  -75,  -91],
                [-132, -102,  -99,  -95,  -97,  -88,  -81, -122],
                [ -94,  -91,  -84,  -78,  -81,  -86,  -75,  -87],
            ]),
            queen: PieceSquareTable([
                [  29,   33,   42,   53,   69,  112,   91,  116],
                [  27,   -7,   31,    8,   23,   83,    1,   69],
                [  19,   24,   23,   37,   53,   73,  102,   68],
                [  27,   26,   28,   26,   30,   42,   50,   57],
                [  46,   42,   40,   29,   33,   48,   51,   61],
                [  51,   53,   47,   44,   46,   52,   64,   70],
                [  49,   49,   54,   49,   53,   63,   69,   55],
                [  49,   48,   53,   55,   57,   34,   35,   41],
            ]),
            king: PieceSquareTable([
                [ -11,   20,   18,   17,   15,    6,    5,   -4],
                [  -7,   47,   61,   36,   40,   51,   24,   -2],
                [   2,   77,   80,   68,   79,   97,   63,   -8],
                [ -13,   44,   71,   24,   52,   59,   46,  -57],
                [  -3,   29,   30,   -8,    0,   -4,   25,  -51],
                [ -32,  -25,  -29,  -54,  -43,  -37,  -20,  -50],
                [ -12,  -19,  -48,  -96,  -64,  -74,  -20,  -14],
                [ -36,   11,  -27, -109,  -46, -101,  -10,   -8],
            ]),
        },
        mobility: Mobility {
            pawn: [-7, -4, 3, 15, 7],
            knight: [-91, -87, -82, -81, -85, -84, -84, -87, -86],
            bishop: [-86, -85, -82, -81, -81, -75, -72, -72, -72, -70, -68, -63, -60, -21],
            rook: [-118, -114, -113, -109, -110, -106, -102, -96, -93, -91, -87, -84, -82, -68, -58],
            queen: [-33, -36, -40, -39, -40, -42, -40, -41, -38, -38, -35, -35, -33, -31, -29, -31, -33, -31, -32, -35, -34, -34, -28, -23, -7, -21, -37, -22],
            king: [-8, -4, -9, -12, -20, -33, -27, -46, -57],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  -8,  -11,   -1,   -2,    8,   -8,  -25,  -13],
            [  71,   48,   39,   32,   18,   20,    5,    7],
            [  29,   21,   19,   12,    4,   -2,   -1,    9],
            [   9,   -6,  -13,  -11,  -13,  -21,  -24,   -4],
            [  -9,  -19,  -21,  -22,  -10,  -19,  -26,    5],
            [ -13,  -18,  -17,  -23,   -3,  -14,    8,   14],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        bishop_pair: 11,
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 181,  181,  164,  134,  127,  137,  180,  182],
                [  51,   50,   42,   38,   52,   33,   51,   44],
                [  40,   40,   34,   13,   22,   22,   33,   27],
                [  30,   38,   25,   25,   25,   24,   26,   17],
                [  22,   31,   27,   27,   32,   31,   25,    8],
                [  24,   28,   27,   31,   39,   35,   24,   -1],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -72,  -25,   19,   -4,   -4,   18,  -19,  -86],
                [ -32,  -14,   -3,   17,   17,  -19,  -14,  -30],
                [ -20,    3,   25,   29,   19,   23,    5,  -21],
                [ -12,    9,   32,   49,   47,   37,   20,   -1],
                [ -16,   12,   34,   39,   41,   35,   16,   -7],
                [ -48,   -4,    7,   20,   17,   16,    3,  -37],
                [ -46,  -12,  -15,   -5,   -3,  -20,  -20,  -41],
                [ -70,  -67,  -30,  -15,  -20,  -29,  -62,  -86],
            ]),
            bishop: PieceSquareTable([
                [  19,   21,   20,   24,   18,   18,    8,   18],
                [   8,    6,   10,   14,    9,    8,   16,   -3],
                [  -5,    6,    5,    3,    5,    3,    4,   -2],
                [   0,    1,    3,   10,   14,    5,    7,    1],
                [  -4,   -4,    6,    9,    6,    5,    0,   -7],
                [  -9,   -3,   -1,   -2,   -2,    1,   -3,   -4],
                [ -12,  -23,  -18,  -10,  -11,  -20,  -19,  -39],
                [ -22,  -13,  -20,   -8,   -9,  -13,  -18,  -23],
            ]),
            rook: PieceSquareTable([
                [ 100,  107,  113,  110,  108,  110,  102,   97],
                [ 111,  117,  115,  114,  114,   85,  102,   88],
                [ 107,   94,  105,   96,   85,   88,   75,   86],
                [ 100,  100,  103,   99,   97,   90,   87,   89],
                [  89,   99,  101,   95,   93,   92,   89,   84],
                [  71,   82,   81,   77,   74,   79,   76,   66],
                [  69,   59,   70,   66,   68,   58,   55,   66],
                [  63,   69,   73,   71,   70,   79,   68,   52],
            ]),
            queen: PieceSquareTable([
                [  78,  101,   99,  105,  116,  110,  111,   98],
                [  64,   91,   89,  125,  155,  121,  158,  116],
                [  48,   50,   75,   91,  124,  134,  103,  129],
                [  32,   59,   59,  100,  121,  130,  121,  108],
                [  10,   43,   39,   84,   81,   81,   74,   92],
                [  -1,   15,   38,   18,   21,   49,   29,   13],
                [ -11,    3,  -17,    6,   -2,  -44,  -63,  -23],
                [  -6,  -22,  -22,    5,  -21,  -16,  -38,    5],
            ]),
            king: PieceSquareTable([
                [-160,  -68,  -38,  -21,  -35,  -30,  -40, -119],
                [ -48,   25,   21,   13,   14,   19,   38,  -23],
                [  -2,   33,   34,   28,   23,   38,   38,   -3],
                [   0,   32,   37,   46,   39,   34,   26,   -2],
                [ -23,   20,   33,   47,   44,   36,   18,  -14],
                [ -15,   17,   26,   40,   36,   31,   12,  -10],
                [ -19,    6,   20,   27,   18,   22,   -2,  -34],
                [ -59,  -33,  -20,  -19,  -56,  -12,  -38,  -87],
            ]),
        },
        mobility: Mobility {
            pawn: [-25, 3, 7, 20, 52],
            knight: [39, 76, 84, 88, 92, 94, 95, 94, 93],
            bishop: [16, 39, 53, 67, 80, 93, 102, 112, 118, 120, 119, 119, 120, 104],
            rook: [86, 100, 112, 119, 126, 131, 136, 137, 144, 151, 152, 157, 158, 153, 147],
            queen: [5, 23, 100, 134, 175, 190, 197, 214, 221, 229, 236, 242, 246, 254, 257, 262, 270, 269, 271, 277, 279, 277, 267, 266, 244, 254, 247, 260],
            king: [18, -5, -3, -2, -2, -3, -12, -13, -14],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  25,   25,   17,   16,   12,   16,   24,   27],
            [ 131,  121,  103,   81,   54,   86,   93,  112],
            [  66,   58,   47,   51,   42,   46,   56,   52],
            [  30,   29,   26,   23,   21,   29,   44,   34],
            [   6,    8,    6,    9,    4,    5,   16,    7],
            [  12,    8,   14,   18,    5,   -1,    2,   11],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        bishop_pair: 70,
    },
};
