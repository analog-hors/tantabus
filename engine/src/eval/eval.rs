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
    passed_pawns: PieceSquareTable
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
                [ 113,  111,  110,  118,  113,   76,  -19,  -16],
                [ -60,  -22,  -17,  -15,    8,   42,    2,  -37],
                [ -52,  -32,  -35,  -23,  -13,  -10,  -16,  -42],
                [ -52,  -35,  -35,  -34,  -23,  -28,  -17,  -38],
                [ -52,  -41,  -42,  -39,  -31,  -29,  -13,  -31],
                [ -58,  -49,  -50,  -51,  -53,  -29,  -13,  -46],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-182,  -50,  -99,  -16,    1,  -58,  -11, -104],
                [ -33,  -26,   15,   40,   29,   67,  -29,    8],
                [ -51,   -8,   26,   38,   69,   69,   21,    4],
                [ -19,   -6,   17,   20,    8,   32,    4,    5],
                [ -23,   -9,    0,   -2,    7,    1,   18,  -11],
                [ -29,  -15,   -9,    0,    3,    1,   -6,  -19],
                [ -49,  -49,  -27,  -19,  -21,  -20,  -36,  -33],
                [-101,  -30,  -58,  -34,  -29,  -31,  -28, -110],
            ]),
            bishop: PieceSquareTable([
                [ -74,  -62,  -94,  -88,  -62,  -97,   -2,  -52],
                [ -55,  -26,  -33,  -49,  -32,   -8,  -37,  -16],
                [ -25,  -14,    1,    1,   25,   31,   19,   -8],
                [ -32,  -10,  -10,   15,   -4,    6,  -15,  -23],
                [ -27,  -12,  -13,   -5,   -1,  -18,  -11,  -12],
                [ -19,   -5,  -12,  -12,  -14,   -7,   -5,   -4],
                [ -14,  -11,  -10,  -22,  -17,  -14,   -3,  -15],
                [ -10,  -14,  -25,  -34,  -33,  -26,  -25,  -18],
            ]),
            rook: PieceSquareTable([
                [ -38,  -42,  -47,  -43,  -35,  -20,   -3,   14],
                [ -74,  -80,  -55,  -36,  -37,    3,  -46,  -10],
                [-100,  -64,  -70,  -49,  -23,  -24,    2,  -42],
                [-103,  -94,  -87,  -70,  -78,  -62,  -63,  -64],
                [-111, -112, -108, -103, -102,  -95,  -72,  -92],
                [-114, -104, -109,  -97,  -97,  -95,  -74,  -90],
                [-131, -101,  -98,  -94,  -96,  -87,  -80, -121],
                [ -93,  -90,  -83,  -77,  -80,  -85,  -74,  -86],
            ]),
            queen: PieceSquareTable([
                [  37,   40,   48,   62,   76,  111,   95,  124],
                [  34,    1,   39,   17,   32,   93,   12,   78],
                [  27,   32,   31,   46,   62,   82,  112,   76],
                [  35,   34,   36,   35,   39,   51,   58,   65],
                [  54,   50,   48,   37,   41,   56,   59,   69],
                [  59,   61,   55,   52,   54,   60,   72,   78],
                [  57,   57,   62,   57,   60,   71,   77,   61],
                [  57,   57,   61,   63,   65,   42,   38,   46],
            ]),
            king: PieceSquareTable([
                [ -10,   12,   11,   11,    9,    3,    2,   -6],
                [  -7,   37,   48,   28,   30,   40,   19,   -2],
                [   2,   66,   70,   55,   65,   84,   57,   -7],
                [  -9,   43,   68,   25,   50,   59,   48,  -50],
                [   0,   33,   35,   -2,    6,    2,   30,  -46],
                [ -25,  -20,  -24,  -51,  -40,  -32,  -15,  -46],
                [  -8,  -14,  -44,  -92,  -61,  -70,  -15,  -10],
                [ -31,   15,  -23, -104,  -42,  -97,   -5,   -3],
            ]),
        },
        mobility: Mobility {
            pawn: [-7, -4, 4, 15, 10],
            knight: [-84, -81, -77, -76, -79, -79, -79, -82, -81],
            bishop: [-70, -69, -67, -66, -66, -59, -56, -56, -55, -53, -52, -46, -43, -1],
            rook: [-112, -109, -108, -104, -105, -101, -97, -91, -88, -86, -82, -79, -77, -63, -53],
            queen: [-13, -15, -19, -18, -19, -21, -19, -20, -17, -17, -14, -14, -12, -11, -9, -11, -13, -11, -12, -15, -15, -15, -8, -6, 0, -18, -33, -21],
            king: [-8, -4, -9, -12, -20, -33, -28, -46, -55],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  -7,  -10,    0,   -1,    9,   -7,  -24,  -12],
            [  72,   49,   39,   33,   19,   21,    7,    8],
            [  29,   21,   19,   12,    5,   -2,    0,   10],
            [   9,   -6,  -13,  -11,  -13,  -20,  -24,   -4],
            [  -9,  -19,  -21,  -22,  -10,  -19,  -25,    5],
            [ -13,  -18,  -17,  -23,   -2,  -14,    9,   15],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 180,  180,  163,  133,  126,  136,  180,  181],
                [  51,   50,   43,   39,   53,   34,   52,   45],
                [  40,   40,   34,   13,   21,   23,   34,   27],
                [  30,   37,   25,   25,   25,   24,   26,   17],
                [  22,   31,   28,   27,   31,   32,   24,    8],
                [  24,   27,   27,   31,   39,   35,   24,   -1],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -73,  -25,   17,   -3,   -3,   18,  -17,  -87],
                [ -32,  -13,   -4,   17,   16,  -20,  -12,  -28],
                [ -18,    2,   23,   26,   17,   21,    5,  -19],
                [ -11,    7,   29,   47,   44,   35,   18,    0],
                [ -16,   11,   31,   36,   38,   32,   16,   -7],
                [ -48,   -6,    3,   17,   13,   12,    1,  -37],
                [ -45,  -11,  -16,   -8,   -5,  -22,  -19,  -41],
                [ -71,  -68,  -30,  -15,  -20,  -29,  -63,  -82],
            ]),
            bishop: PieceSquareTable([
                [  18,   21,   19,   23,   16,   16,    6,   15],
                [  10,    5,   11,   13,    9,    6,   14,   -4],
                [  -4,    6,    4,    4,    4,    3,    3,    1],
                [   2,    1,    3,    8,   14,    4,    8,    2],
                [  -3,   -2,    6,   10,    5,    6,    0,   -5],
                [  -6,   -3,    0,   -1,    0,    1,   -2,   -2],
                [ -11,  -20,  -17,   -7,  -10,  -19,  -18,  -35],
                [ -18,  -12,  -15,   -6,   -6,  -11,  -18,  -21],
            ]),
            rook: PieceSquareTable([
                [  99,  105,  111,  108,  106,  107,  100,   95],
                [ 110,  116,  114,  112,  112,   83,  100,   87],
                [ 106,   93,  104,   94,   84,   87,   73,   85],
                [  98,   98,  102,   98,   96,   89,   86,   88],
                [  87,   98,  100,   94,   92,   91,   88,   83],
                [  70,   80,   80,   75,   72,   78,   75,   64],
                [  68,   57,   69,   64,   66,   57,   54,   64],
                [  61,   68,   71,   70,   68,   77,   67,   51],
            ]),
            queen: PieceSquareTable([
                [  72,   95,   94,   98,  110,  110,  108,   92],
                [  57,   84,   82,  118,  147,  113,  145,  106],
                [  41,   43,   67,   84,  117,  126,   95,  122],
                [  25,   52,   51,   92,  113,  122,  114,  101],
                [   3,   35,   32,   76,   74,   74,   67,   85],
                [  -8,    8,   30,   11,   13,   41,   22,    6],
                [ -19,   -5,  -24,   -1,   -9,  -51,  -70,  -25],
                [ -14,  -29,  -30,   -2,  -29,  -23,  -35,    5],
            ]),
            king: PieceSquareTable([
                [-139,  -63,  -36,  -19,  -33,  -28,  -38, -105],
                [ -48,   26,   22,   13,   16,   20,   37,  -24],
                [  -2,   34,   35,   30,   25,   40,   39,   -4],
                [  -2,   31,   37,   45,   38,   34,   25,   -4],
                [ -24,   18,   31,   45,   43,   34,   16,  -16],
                [ -17,   15,   24,   38,   34,   29,   10,  -12],
                [ -20,    4,   18,   26,   16,   20,   -4,  -35],
                [ -60,  -34,  -21,  -20,  -57,  -13,  -39,  -88],
            ]),
        },
        mobility: Mobility {
            pawn: [-26, 2, 6, 20, 54],
            knight: [23, 68, 77, 82, 87, 89, 91, 92, 92],
            bishop: [17, 41, 55, 69, 83, 97, 106, 115, 121, 124, 123, 122, 123, 106],
            rook: [79, 95, 105, 112, 120, 125, 130, 131, 138, 144, 146, 151, 151, 147, 141],
            queen: [3, 16, 73, 111, 155, 170, 177, 195, 202, 210, 217, 223, 227, 236, 239, 244, 252, 252, 254, 260, 262, 260, 250, 252, 237, 248, 245, 262],
            king: [16, -6, -3, -2, -2, -2, -11, -12, -13],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  24,   24,   17,   15,   11,   16,   24,   27],
            [ 130,  121,  102,   79,   53,   85,   92,  111],
            [  66,   58,   47,   51,   42,   46,   56,   51],
            [  30,   29,   26,   23,   21,   29,   44,   34],
            [   6,    8,    6,    9,    4,    5,   16,    7],
            [  12,    8,   14,   17,    4,   -1,    2,   11],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
    },
};
