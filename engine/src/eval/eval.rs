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
                [  71,   85,   66,   92,   73,   53,  -21,  -39],
                [ -45,  -29,   -9,   -4,   -3,   35,   27,  -14],
                [ -51,  -28,  -26,  -23,   -1,  -11,   -9,  -30],
                [ -55,  -31,  -28,  -13,  -11,  -17,  -15,  -38],
                [ -50,  -34,  -31,  -29,  -15,  -19,    2,  -25],
                [ -60,  -40,  -43,  -48,  -35,  -25,   -7,  -54],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-157,  -73, -100,  -26,   -2,  -73,  -29, -110],
                [ -32,  -14,   15,   32,   18,   69,  -17,   13],
                [ -29,   10,   39,   47,   85,   72,   35,    3],
                [ -21,   -5,   21,   43,   25,   48,    6,   10],
                [ -23,  -10,   10,    9,   21,   18,   17,  -11],
                [ -32,  -12,    3,    5,   16,   16,   10,  -12],
                [ -53,  -42,  -26,  -11,  -12,  -16,  -36,  -36],
                [ -90,  -33,  -48,  -32,  -22,  -32,  -29,  -69],
            ]),
            bishop: PieceSquareTable([
                [ -46,  -70,  -91, -101,  -77,  -93,  -14,  -62],
                [ -31,  -10,  -19,  -47,   -7,   -5,  -13,  -22],
                [ -27,   -6,   -4,   14,    8,   26,   15,   -2],
                [ -27,  -17,   -1,   14,    6,    4,  -18,  -28],
                [ -15,  -15,  -10,    7,    5,   -9,  -13,   -7],
                [  -8,    5,   -1,   -3,   -4,    8,    8,    9],
                [  -9,   -3,    1,  -20,   -7,   -7,    3,  -19],
                [ -21,    2,  -11,  -23,  -15,  -31,   -9,  -18],
            ]),
            rook: PieceSquareTable([
                [ -53,  -61,  -65,  -62,  -43,  -31,  -18,   -2],
                [ -67,  -69,  -51,  -32,  -46,  -19,  -26,    1],
                [ -93,  -73,  -71,  -66,  -38,  -41,    4,  -24],
                [ -99,  -87,  -86,  -78,  -77,  -63,  -60,  -58],
                [-104, -102,  -93,  -87,  -84,  -88,  -65,  -80],
                [ -99,  -95,  -85,  -84,  -77,  -71,  -40,  -63],
                [-101,  -93,  -75,  -77,  -69,  -75,  -60,  -91],
                [ -80,  -78,  -68,  -61,  -54,  -71,  -62,  -77],
            ]),
            queen: PieceSquareTable([
                [   5,    5,   22,   44,   43,   78,   63,   65],
                [  35,   11,   18,   -3,   11,   49,   28,   75],
                [  28,   21,   22,   32,   48,   62,   83,   73],
                [  20,   20,   25,   24,   26,   34,   39,   46],
                [  38,   30,   30,   33,   33,   38,   49,   56],
                [  48,   48,   43,   38,   41,   58,   68,   66],
                [  41,   42,   50,   51,   48,   55,   62,   64],
                [  38,   37,   46,   51,   51,   29,   53,   39],
            ]),
            king: PieceSquareTable([
                [  -3,   21,   18,   12,   14,   10,   12,    0],
                [ -13,   40,   49,   43,   43,   55,   30,    4],
                [ -13,   71,   64,   51,   70,   98,   66,   -8],
                [ -29,   19,   33,  -15,   -4,   21,   14,  -79],
                [ -29,   -1,  -34,  -82,  -81,  -53,  -41, -100],
                [ -39,    8,  -44,  -67,  -60,  -55,  -14,  -70],
                [  31,   33,   11,  -15,  -21,   -4,   38,   12],
                [   4,   51,   24,  -57,    1,  -36,   33,   17],
            ]),
        },
        mobility: Mobility {
            pawn: [-1, 6, 15, 37, 5],
            knight: [-66, -52, -42, -38, -36, -36, -36, -39, -37],
            bishop: [-63, -55, -49, -43, -36, -28, -23, -20, -21, -18, -17, -14, -32, -15],
            rook: [-97, -84, -80, -77, -77, -71, -66, -58, -54, -49, -46, -41, -41, -37, -37],
            queen: [-50, -64, -72, -71, -69, -65, -61, -62, -61, -59, -58, -56, -57, -54, -54, -53, -49, -53, -51, -48, -39, -38, -30, -25, 3, 3, -24, -14],
            king: [25, 36, 20, 2, -18, -45, -55, -80, -101],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [ -49,  -36,  -44,  -27,  -30,  -30,  -26,  -35],
            [  24,   32,   25,   13,   21,    9,  -21,  -36],
            [  12,    9,   16,   13,   -3,   11,  -28,  -15],
            [  -2,  -11,  -22,  -11,  -16,   -6,  -23,  -12],
            [ -12,  -21,  -27,  -19,  -16,  -17,  -21,    2],
            [ -17,   -9,  -18,  -23,   -6,   -4,    8,   -5],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        bishop_pair: 22,
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 197,  189,  187,  145,  142,  152,  193,  201],
                [  46,   52,   30,   40,   27,   17,   52,   36],
                [  41,   38,   26,   15,   15,   15,   30,   18],
                [  27,   34,   22,   22,   21,   18,   25,   10],
                [  26,   33,   20,   25,   27,   20,   21,    6],
                [  21,   30,   19,   29,   34,   22,   17,    5],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -85,  -43,    0,  -24,  -16,  -24,  -39, -100],
                [ -26,  -10,   -6,   -7,  -14,  -24,  -13,  -43],
                [ -16,   -7,   13,   18,    5,   -8,  -17,  -31],
                [  -8,    8,   24,   30,   26,   19,    4,  -14],
                [   0,    4,   27,   24,   30,   20,    5,   -8],
                [ -13,    1,   13,   18,   15,   12,    1,  -13],
                [ -33,  -13,   -6,   -6,   -6,   -8,  -24,  -24],
                [ -48,  -45,  -21,  -21,  -22,  -25,  -40,  -59],
            ]),
            bishop: PieceSquareTable([
                [  -2,    1,    2,    6,   -3,   -2,  -15,  -10],
                [ -18,  -14,  -10,   -6,  -18,  -18,   -6,  -24],
                [  -5,  -12,   -7,  -15,  -11,   -9,  -16,  -10],
                [  -9,   -5,   -7,    5,   -2,   -6,   -9,  -10],
                [ -11,   -2,    2,   -2,   -3,   -1,   -3,  -15],
                [   2,    1,   -2,   -4,   -2,    3,    1,   -9],
                [ -10,  -18,  -24,   -8,  -12,  -13,  -11,  -20],
                [ -21,  -10,  -15,  -11,  -12,   -1,  -22,  -36],
            ]),
            rook: PieceSquareTable([
                [  86,   90,  102,   96,   89,   83,   79,   75],
                [  87,   97,  100,   90,   91,   79,   77,   66],
                [  87,   85,   88,   84,   73,   64,   60,   58],
                [  91,   84,   92,   87,   74,   66,   68,   66],
                [  89,   88,   85,   82,   78,   77,   72,   71],
                [  82,   79,   75,   76,   71,   66,   53,   57],
                [  73,   76,   75,   74,   66,   65,   54,   63],
                [  72,   72,   77,   72,   64,   74,   64,   61],
            ]),
            queen: PieceSquareTable([
                [  62,   76,   92,   90,   90,   72,   61,   55],
                [  42,   63,   91,  124,  136,   96,  108,   87],
                [  45,   52,   76,   94,  103,   94,   55,   72],
                [  51,   55,   65,   82,   97,   98,   86,   77],
                [  36,   58,   54,   71,   73,   72,   70,   69],
                [  20,   41,   48,   44,   50,   55,   48,   37],
                [  15,   21,   18,   23,   31,    4,  -29,  -25],
                [  12,   15,   12,   33,   13,   15,  -19,    3],
            ]),
            king: PieceSquareTable([
                [-123,  -61,  -39,  -19,  -30,  -17,  -19, -111],
                [ -31,    9,   11,   21,   30,   39,   38,   -2],
                [ -16,   14,   28,   38,   44,   41,   37,    5],
                [ -16,   16,   34,   48,   47,   40,   28,    8],
                [ -28,    8,   34,   52,   53,   40,   23,    5],
                [ -24,    5,   24,   37,   36,   30,   12,    0],
                [ -40,  -13,    1,   11,   14,    6,  -17,  -33],
                [ -72,  -57,  -33,  -22,  -40,  -22,  -51,  -74],
            ]),
        },
        mobility: Mobility {
            pawn: [-31, -7, -2, -14, 52],
            knight: [51, 57, 57, 51, 53, 58, 57, 56, 51],
            bishop: [-5, 11, 18, 31, 43, 56, 59, 67, 74, 72, 72, 71, 91, 71],
            rook: [55, 70, 73, 77, 83, 84, 85, 88, 94, 100, 101, 105, 110, 113, 111],
            queen: [5, 26, 109, 148, 165, 171, 178, 198, 207, 209, 218, 222, 232, 238, 244, 250, 252, 264, 273, 274, 273, 278, 275, 282, 263, 281, 265, 272],
            king: [13, -24, -19, -6, -4, 0, 1, 4, -1],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  42,   33,   40,   27,   27,   31,   37,   47],
            [ 146,  142,  117,   73,   81,  107,  115,  145],
            [  74,   70,   53,   45,   43,   51,   69,   76],
            [  40,   33,   27,   17,   21,   23,   42,   41],
            [   5,    8,   11,    1,    0,    5,   21,    8],
            [   3,    5,   12,    4,  -12,   -4,    2,    5],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        bishop_pair: 65,
    },
};
