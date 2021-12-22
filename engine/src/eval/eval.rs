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
                [ 111,  110,  109,  117,  111,   75,  -21,  -19],
                [ -65,  -27,  -21,  -19,    3,   38,   -2,  -42],
                [ -58,  -37,  -41,  -28,  -19,  -16,  -22,  -48],
                [ -59,  -41,  -42,  -40,  -29,  -33,  -22,  -44],
                [ -58,  -47,  -49,  -45,  -37,  -35,  -18,  -36],
                [ -62,  -54,  -55,  -56,  -58,  -32,  -16,  -51],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-243, -101, -159,  -62,  -44, -112,  -29, -159],
                [ -87,  -79,  -39,  -13,  -23,   15,  -80,  -44],
                [-104,  -62,  -27,  -16,   16,   18,  -32,  -50],
                [ -74,  -60,  -37,  -35,  -47,  -23,  -50,  -49],
                [ -78,  -64,  -55,  -57,  -48,  -54,  -37,  -66],
                [ -84,  -70,  -64,  -55,  -51,  -53,  -61,  -76],
                [-105, -105,  -82,  -74,  -77,  -73,  -91,  -90],
                [-163,  -87, -114,  -89,  -85,  -87,  -86, -165],
            ]),
            bishop: PieceSquareTable([
                [-128, -121, -153, -143, -116, -152,  -36, -108],
                [-109,  -81,  -87, -103,  -86,  -61,  -91,  -69],
                [ -79,  -67,  -53,  -53,  -28,  -20,  -36,  -62],
                [ -87,  -65,  -63,  -37,  -57,  -48,  -70,  -78],
                [ -82,  -67,  -69,  -59,  -55,  -74,  -66,  -66],
                [ -74,  -61,  -66,  -68,  -69,  -62,  -60,  -59],
                [ -68,  -65,  -65,  -78,  -72,  -69,  -58,  -70],
                [ -65,  -69,  -77,  -88,  -88,  -80,  -81,  -73],
            ]),
            rook: PieceSquareTable([
                [-123, -126, -129, -123, -119, -106,  -91,  -74],
                [-161, -167, -141, -121, -123,  -82, -134,  -98],
                [-188, -152, -157, -135, -109, -110,  -86, -131],
                [-192, -183, -175, -157, -165, -151, -153, -154],
                [-201, -201, -198, -192, -192, -185, -162, -183],
                [-205, -195, -200, -188, -189, -187, -164, -180],
                [-221, -192, -189, -185, -187, -177, -170, -211],
                [-184, -180, -173, -166, -170, -176, -164, -176],
            ]),
            queen: PieceSquareTable([
                [  61,   65,   77,   89,  104,  158,  129,  151],
                [  58,   26,   65,   42,   58,  119,   34,  102],
                [  52,   57,   57,   70,   89,  108,  139,  101],
                [  60,   60,   63,   61,   67,   77,   84,   90],
                [  78,   76,   74,   64,   69,   82,   84,   93],
                [  83,   85,   80,   77,   80,   84,   96,  102],
                [  80,   81,   86,   82,   85,   95,  101,   87],
                [  81,   81,   88,   89,   90,   64,   69,   76],
            ]),
            king: PieceSquareTable([
                [  -6,   44,   38,   35,   31,   17,   17,    6],
                [  -7,   69,   83,   53,   62,   72,   32,    1],
                [   6,   89,   91,   82,   93,  106,   65,  -10],
                [ -13,   42,   67,   13,   45,   48,   37,  -61],
                [  -5,   22,   23,  -14,   -6,  -10,   16,  -56],
                [ -39,  -34,  -37,  -62,  -51,  -44,  -28,  -57],
                [ -19,  -28,  -57, -106,  -73,  -84,  -30,  -22],
                [ -44,    4,  -34, -118,  -54, -110,  -17,  -15],
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
            [  -8,  -10,    0,   -2,    8,   -7,  -26,  -15],
            [  73,   50,   41,   34,   20,   22,    8,    9],
            [  30,   22,   20,   13,    5,   -1,    1,   11],
            [  10,   -6,  -13,  -10,  -13,  -19,  -24,   -3],
            [  -9,  -19,  -21,  -22,  -10,  -18,  -24,    6],
            [ -13,  -18,  -16,  -22,   -1,  -13,   11,   16],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 168,  168,  151,  121,  114,  124,  167,  170],
                [  21,   21,   16,   11,   27,    5,   22,   14],
                [  18,   19,   13,   -7,    2,    0,   12,    6],
                [  12,   18,    5,    4,    4,    5,    6,   -3],
                [   2,   11,    6,    5,   10,   11,    4,  -12],
                [  -7,   -3,   -1,    2,   10,    6,   -8,  -33],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -16,   31,   78,   53,   53,   77,   30,  -41],
                [  25,   45,   54,   74,   74,   38,   46,   27],
                [  39,   59,   75,   78,   69,   72,   61,   38],
                [  45,   65,   82,  101,   99,   87,   77,   56],
                [  45,   71,   87,   90,   93,   91,   76,   53],
                [  12,   53,   57,   72,   68,   68,   60,   23],
                [  12,   46,   42,   49,   52,   35,   37,   14],
                [ -14,  -16,   26,   39,   35,   26,  -10,  -40],
            ]),
            bishop: PieceSquareTable([
                [  58,   62,   61,   65,   57,   57,   42,   55],
                [  46,   44,   50,   53,   49,   45,   51,   35],
                [  35,   44,   37,   39,   39,   34,   40,   39],
                [  41,   41,   39,   37,   43,   36,   49,   41],
                [  39,   38,   43,   41,   32,   46,   41,   35],
                [  34,   37,   36,   34,   37,   38,   37,   38],
                [  28,   15,   21,   30,   27,   18,   17,   -2],
                [  15,   25,   14,   31,   29,   21,   17,   14],
            ]),
            rook: PieceSquareTable([
                [ 174,  179,  184,  181,  181,  186,  179,  175],
                [ 188,  194,  191,  189,  190,  162,  180,  167],
                [ 185,  172,  182,  173,  162,  167,  153,  165],
                [ 177,  177,  181,  176,  176,  169,  166,  168],
                [ 169,  179,  180,  174,  173,  175,  170,  164],
                [ 151,  162,  161,  157,  154,  160,  156,  145],
                [ 147,  138,  149,  145,  146,  137,  134,  143],
                [ 141,  147,  149,  148,  147,  154,  145,  129],
            ]),
            queen: PieceSquareTable([
                [ 167,  188,  182,  186,  201,  187,  196,  184],
                [ 155,  177,  172,  206,  236,  205,  245,  204],
                [ 137,  135,  151,  167,  198,  212,  185,  219],
                [ 120,  142,  134,  169,  188,  204,  208,  196],
                [ 100,  126,  118,  157,  153,  168,  164,  183],
                [  89,  105,  120,  102,  105,  136,  120,  104],
                [  77,   88,   71,   92,   84,   44,   24,   60],
                [  80,   62,   58,   84,   62,   73,   42,   81],
            ]),
            king: PieceSquareTable([
                [-172,  -65,  -36,  -18,  -32,  -26,  -36, -126],
                [ -45,   20,   16,    8,    9,   15,   35,  -20],
                [  -1,   28,   29,   23,   18,   34,   35,   -1],
                [   0,   30,   34,   45,   37,   32,   25,    1],
                [ -19,   19,   32,   46,   43,   36,   18,   -9],
                [ -10,   17,   26,   39,   35,   32,   13,   -5],
                [ -14,    6,   19,   27,   17,   21,   -2,  -30],
                [ -57,  -29,  -17,  -14,  -51,   -8,  -35,  -86],
            ]),
        },
        mobility: PieceEvalSet {
            pawn: 19,
            knight: 2,
            bishop: 10,
            rook: 4,
            queen: 5,
            king: -2
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  13,   12,    4,    3,   -1,    3,   12,   15],
            [ 138,  128,  107,   85,   57,   91,  100,  119],
            [  66,   58,   47,   50,   41,   48,   56,   52],
            [  29,   29,   26,   23,   21,   28,   43,   33],
            [   7,    9,    7,   10,    4,    4,   16,    7],
            [  14,    9,   13,   17,    3,   -2,    2,   12],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
    },
};
