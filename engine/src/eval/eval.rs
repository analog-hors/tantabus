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
pub struct PhasedEval {
    piece_tables: PieceEvalSet<PieceSquareTable>,
    mobility: PieceEvalSet<i16>,
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
        let phase = Self::game_phase(&board);
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
            let midgame_mobility = *self.midgame.mobility.get(piece);
            let endgame_mobility = *self.endgame.mobility.get(piece);

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
                let mobility = approx_moves.popcnt() as i16;
                midgame_value += mobility * midgame_mobility;
                endgame_value += mobility * endgame_mobility;

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
                [ 117,  117,  115,  122,  117,   83,  -17,  -14],
                [ -62,  -24,  -18,  -16,    6,   42,    2,  -39],
                [ -56,  -34,  -38,  -25,  -15,  -12,  -18,  -45],
                [ -56,  -38,  -38,  -37,  -26,  -30,  -19,  -41],
                [ -56,  -44,  -45,  -42,  -33,  -31,  -14,  -33],
                [ -60,  -51,  -52,  -53,  -55,  -29,  -13,  -48],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-215,  -54, -107,  -22,   -8,  -64,  -10, -115],
                [ -55,  -47,   -4,   23,   14,   52,  -43,   -8],
                [ -70,  -28,    8,   20,   53,   56,    3,  -16],
                [ -41,  -26,   -2,    0,  -11,   13,  -16,  -15],
                [ -45,  -29,  -20,  -22,  -13,  -19,   -2,  -33],
                [ -51,  -36,  -30,  -20,  -17,  -19,  -27,  -43],
                [ -73,  -73,  -48,  -40,  -44,  -40,  -59,  -57],
                [-114,  -55,  -81,  -57,  -53,  -55,  -54, -132],
            ]),
            bishop: PieceSquareTable([
                [ -97,  -76, -109, -101,  -77, -114,   -8,  -69],
                [ -77,  -49,  -54,  -68,  -52,  -26,  -57,  -36],
                [ -47,  -34,  -19,  -19,    6,   15,   -1,  -29],
                [ -54,  -32,  -30,   -4,  -23,  -15,  -37,  -45],
                [ -49,  -34,  -36,  -25,  -21,  -41,  -33,  -33],
                [ -41,  -28,  -34,  -35,  -36,  -29,  -27,  -26],
                [ -35,  -33,  -32,  -45,  -40,  -36,  -25,  -37],
                [ -33,  -36,  -46,  -55,  -56,  -47,  -48,  -40],
            ]),
            rook: PieceSquareTable([
                [ -78,  -80,  -85,  -77,  -73,  -49,  -35,  -24],
                [-116, -122,  -95,  -75,  -76,  -34,  -86,  -52],
                [-144, -107, -111,  -89,  -62,  -64,  -38,  -85],
                [-148, -138, -130, -111, -120, -106, -107, -109],
                [-157, -157, -153, -148, -147, -141, -117, -138],
                [-161, -150, -156, -143, -144, -142, -119, -136],
                [-177, -147, -144, -141, -142, -133, -126, -166],
                [-140, -136, -129, -122, -126, -132, -119, -133],
            ]),
            queen: PieceSquareTable([
                [  66,   71,   79,   93,  108,  140,  124,  155],
                [  63,   30,   71,   48,   64,  128,   43,  110],
                [  56,   62,   62,   76,   96,  116,  150,  108],
                [  64,   65,   68,   67,   74,   84,   90,   96],
                [  84,   82,   80,   70,   75,   88,   90,   99],
                [  88,   91,   86,   83,   85,   90,  102,  108],
                [  86,   87,   92,   87,   90,  101,  106,   91],
                [  87,   87,   93,   95,   95,   69,   66,   76],
            ]),
            king: PieceSquareTable([
                [ -10,   13,   13,   13,   11,    5,    4,   -4],
                [  -5,   38,   47,   27,   31,   41,   20,   -1],
                [   4,   66,   69,   53,   62,   83,   55,   -6],
                [  -4,   44,   66,   21,   47,   53,   44,  -47],
                [   4,   34,   36,   -2,    7,    2,   28,  -43],
                [ -23,  -20,  -24,  -49,  -38,  -32,  -16,  -45],
                [  -6,  -16,  -45,  -94,  -61,  -72,  -17,   -9],
                [ -32,   16,  -22, -107,  -43,  -99,   -5,   -3],
            ]),
        },
        mobility: PieceEvalSet {
            pawn: 5,
            knight: -1,
            bishop: 3,
            rook: 4,
            queen: 0,
            king: -5
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  -3,   -4,    5,    3,   13,    0,  -22,  -10],
            [  77,   54,   45,   37,   22,   26,   13,   12],
            [  32,   23,   21,   14,    7,    0,    4,   14],
            [  10,   -5,  -12,  -10,  -12,  -18,  -23,   -2],
            [ -10,  -19,  -21,  -22,   -9,  -17,  -23,    7],
            [ -14,  -18,  -15,  -22,    1,  -11,   14,   18],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 163,  162,  146,  117,  110,  119,  163,  164],
                [  18,   17,   13,    9,   23,    2,   18,   11],
                [  15,   15,   10,  -10,   -2,   -4,    8,    2],
                [   9,   14,    1,    1,    1,    1,    2,   -6],
                [  -1,    7,    3,    2,    7,    8,    1,  -15],
                [  -9,   -6,   -4,    0,    7,    2,  -11,  -35],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -45,   -4,   40,   18,   18,   39,    0,  -74],
                [  -7,   12,   17,   37,   36,    1,   12,   -6],
                [   5,   23,   36,   39,   29,   32,   25,    4],
                [  12,   29,   43,   61,   59,   48,   41,   23],
                [  12,   34,   48,   51,   54,   52,   39,   19],
                [ -22,   17,   19,   34,   30,   30,   23,  -10],
                [ -21,   13,    6,   14,   17,   -1,    3,  -19],
                [ -54,  -48,   -7,    7,    2,   -7,  -42,  -69],
            ]),
            bishop: PieceSquareTable([
                [  27,   28,   27,   32,   25,   24,   11,   21],
                [  15,   13,   18,   21,   17,   13,   19,    3],
                [   4,   12,    5,    7,    6,    1,    8,    7],
                [  10,    9,    6,    4,   11,    4,   17,   10],
                [   8,    6,   10,    8,   -1,   13,    9,    3],
                [   2,    5,    4,    2,    4,    5,    5,    6],
                [  -4,  -16,  -11,   -1,   -4,  -14,  -14,  -33],
                [ -16,   -6,  -17,    0,   -2,  -10,  -14,  -17],
            ]),
            rook: PieceSquareTable([
                [ 127,  132,  137,  133,  134,  135,  129,  126],
                [ 142,  148,  144,  142,  142,  115,  133,  120],
                [ 139,  125,  136,  126,  115,  120,  106,  119],
                [ 131,  131,  134,  130,  130,  123,  120,  122],
                [ 123,  133,  134,  128,  127,  129,  123,  119],
                [ 105,  116,  115,  111,  108,  114,  110,  100],
                [ 102,   92,  103,   99,  101,   92,   88,   97],
                [  96,  101,  104,  102,  101,  109,  100,   84],
            ]),
            queen: PieceSquareTable([
                [ 109,  129,  124,  125,  141,  144,  143,  125],
                [  99,  119,  111,  145,  174,  141,  174,  142],
                [  81,   76,   90,  105,  133,  149,  121,  160],
                [  63,   82,   72,  105,  124,  142,  148,  138],
                [  42,   66,   57,   95,   91,  107,  104,  126],
                [  31,   47,   60,   41,   45,   77,   62,   46],
                [  19,   30,   13,   34,   27,  -14,  -34,    6],
                [  22,    3,    0,   27,    5,   16,   -2,   34],
            ]),
            king: PieceSquareTable([
                [-133,  -54,  -28,  -12,  -25,  -22,  -31, -100],
                [ -46,   23,   21,   12,   15,   19,   35,  -21],
                [  -2,   30,   31,   26,   22,   36,   35,   -3],
                [  -3,   27,   32,   41,   35,   29,   22,   -3],
                [ -22,   15,   28,   42,   39,   32,   14,  -13],
                [ -14,   13,   22,   35,   31,   28,    9,   -9],
                [ -18,    3,   16,   23,   14,   18,   -5,  -33],
                [ -60,  -32,  -20,  -17,  -54,  -11,  -38,  -88],
            ]),
        },
        mobility: PieceEvalSet {
            pawn: 18,
            knight: 3,
            bishop: 10,
            rook: 4,
            queen: 6,
            king: -2
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [   7,    6,   -1,   -1,   -5,   -2,    7,    9],
            [ 133,  123,  102,   80,   54,   87,   95,  115],
            [  64,   57,   45,   49,   40,   46,   54,   50],
            [  28,   28,   25,   22,   20,   27,   42,   32],
            [   7,    9,    6,   10,    3,    4,   15,    6],
            [  14,    9,   12,   16,    1,   -3,    1,   11],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
    },
};
