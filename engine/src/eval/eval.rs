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
    bishop_pair: i16,
    doubled_pawns: i16
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

                    let doubled = !(our_pawns & front_span).is_empty();

                    let mut blocker_mask = front_span;
                    for attack in get_pawn_attacks(square, color) {
                        let telestop = Square::new(attack.file(), promotion_rank);
                        let front_span = get_between_rays(attack, telestop);
                        blocker_mask |= front_span | attack.bitboard();
                    }
                    let passed = (their_pawns & blocker_mask).is_empty() && !doubled;

                    if passed {
                        midgame_value += self.midgame.passed_pawns.get(color, square);
                        endgame_value += self.endgame.passed_pawns.get(color, square);
                    }

                    if doubled {
                        midgame_value += self.midgame.doubled_pawns;
                        endgame_value += self.endgame.doubled_pawns;
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
                [ 110,  110,  109,  117,  112,   75,  -17,  -17],
                [ -59,  -21,  -13,  -11,   11,   47,    7,  -37],
                [ -52,  -32,  -34,  -21,  -11,   -8,  -15,  -43],
                [ -53,  -36,  -36,  -34,  -24,  -27,  -18,  -41],
                [ -53,  -43,  -42,  -39,  -31,  -26,  -15,  -34],
                [ -58,  -50,  -50,  -51,  -53,  -27,  -13,  -48],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-178,  -60, -109,  -19,    2,  -68,  -16, -110],
                [ -34,  -27,   14,   40,   29,   65,  -28,    9],
                [ -50,   -7,   27,   37,   68,   71,   23,    6],
                [ -19,   -6,   17,   20,    8,   32,    4,    6],
                [ -24,  -10,    0,   -1,    6,    0,   16,  -12],
                [ -30,  -16,  -10,    0,    2,    0,   -8,  -21],
                [ -48,  -50,  -27,  -19,  -21,  -20,  -37,  -32],
                [-110,  -30,  -58,  -33,  -28,  -31,  -29, -106],
            ]),
            bishop: PieceSquareTable([
                [ -78,  -74, -106, -101,  -70, -106,   -5,  -61],
                [ -61,  -30,  -37,  -54,  -36,  -12,  -43,  -17],
                [ -27,  -17,    0,   -2,   22,   28,   17,   -9],
                [ -35,  -11,  -13,   13,   -6,    4,  -17,  -25],
                [ -30,  -16,  -15,   -7,   -3,  -21,  -15,  -15],
                [ -22,   -9,  -14,  -13,  -17,  -10,   -9,   -7],
                [ -15,  -13,  -11,  -25,  -19,  -17,   -4,  -17],
                [ -12,  -16,  -28,  -35,  -35,  -27,  -28,  -19],
            ]),
            rook: PieceSquareTable([
                [ -40,  -44,  -51,  -46,  -40,  -28,   -7,   12],
                [ -75,  -81,  -56,  -38,  -39,    1,  -48,  -10],
                [ -99,  -63,  -70,  -50,  -25,  -21,    3,  -42],
                [-102,  -93,  -86,  -70,  -78,  -60,  -62,  -63],
                [-111, -113, -109, -103, -103,  -94,  -73,  -94],
                [-115, -106, -110,  -97,  -98,  -96,  -75,  -92],
                [-132, -102,  -98,  -94,  -96,  -86,  -80, -122],
                [ -94,  -90,  -83,  -77,  -80,  -85,  -73,  -87],
            ]),
            queen: PieceSquareTable([
                [  25,   29,   38,   49,   66,  111,   89,  112],
                [  22,  -12,   27,    3,   19,   78,   -5,   65],
                [  16,   20,   19,   33,   48,   71,  100,   66],
                [  23,   22,   24,   22,   26,   38,   47,   54],
                [  42,   38,   36,   26,   29,   44,   46,   56],
                [  47,   48,   43,   41,   43,   47,   59,   64],
                [  45,   45,   51,   46,   49,   60,   65,   51],
                [  45,   45,   50,   51,   54,   31,   34,   39],
            ]),
            king: PieceSquareTable([
                [ -10,   25,   22,   21,   18,    8,    7,   -2],
                [  -8,   52,   68,   41,   45,   56,   26,   -2],
                [   2,   81,   85,   74,   85,  102,   65,   -9],
                [ -15,   44,   72,   23,   52,   58,   46,  -60],
                [  -6,   27,   28,   -9,   -2,   -6,   22,  -54],
                [ -36,  -28,  -31,  -56,  -45,  -39,  -23,  -54],
                [ -15,  -21,  -50,  -98,  -66,  -76,  -23,  -16],
                [ -38,    9,  -29, -111,  -49, -103,  -12,   -9],
            ]),
        },
        mobility: Mobility {
            pawn: [-2, 0, 6, 18, 5],
            knight: [-92, -87, -83, -82, -85, -84, -84, -87, -86],
            bishop: [-85, -84, -81, -80, -80, -73, -70, -71, -70, -68, -67, -62, -60, -22],
            rook: [-116, -112, -111, -107, -108, -104, -100, -94, -91, -88, -84, -82, -80, -66, -55],
            queen: [-44, -45, -50, -49, -50, -52, -50, -50, -48, -48, -44, -44, -42, -41, -39, -41, -43, -41, -41, -45, -44, -44, -37, -32, -12, -23, -38, -22],
            king: [-8, -4, -9, -11, -19, -33, -27, -46, -59],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  -9,  -10,   -1,   -2,    8,   -8,  -22,  -13],
            [  71,   49,   36,   30,   18,   17,    6,   10],
            [  28,   23,   18,   12,    6,   -3,    2,   11],
            [   8,   -7,  -14,  -12,  -16,  -22,  -24,   -4],
            [ -12,  -21,  -25,  -27,  -17,  -26,  -28,    3],
            [ -16,  -20,  -22,  -29,  -10,  -21,    6,   12],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        bishop_pair: 11,
        doubled_pawns: -15,
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 179,  179,  162,  132,  125,  136,  178,  180],
                [  53,   55,   47,   39,   54,   40,   57,   46],
                [  38,   41,   34,   13,   22,   26,   35,   24],
                [  26,   36,   23,   22,   23,   24,   25,   13],
                [  19,   29,   28,   27,   31,   32,   22,    5],
                [  25,   31,   32,   35,   42,   40,   27,    0],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -73,  -24,   19,   -3,   -4,   19,  -19,  -86],
                [ -32,  -14,   -2,   18,   18,  -20,  -13,  -30],
                [ -19,    5,   26,   29,   19,   26,    7,  -20],
                [ -11,   10,   33,   50,   48,   38,   22,    0],
                [ -17,   12,   34,   39,   41,   34,   16,   -8],
                [ -49,   -4,    7,   21,   17,   12,    0,  -39],
                [ -45,  -11,  -14,   -4,   -3,  -19,  -19,  -40],
                [ -68,  -66,  -29,  -14,  -20,  -28,  -61,  -87],
            ]),
            bishop: PieceSquareTable([
                [  19,   22,   21,   25,   19,   19,    8,   19],
                [   9,    6,   10,   14,    9,    8,   16,   -3],
                [  -3,    7,    6,    4,    5,    6,    7,    0],
                [   0,    3,    5,   11,   15,    6,    8,    2],
                [  -4,   -3,    7,   11,    6,    4,   -1,   -8],
                [  -9,   -3,    0,   -1,   -1,    0,   -5,   -5],
                [ -12,  -22,  -17,  -10,  -10,  -20,  -18,  -38],
                [ -22,  -13,  -19,   -8,   -9,  -11,  -18,  -22],
            ]),
            rook: PieceSquareTable([
                [ 102,  108,  115,  112,  110,  113,  103,   98],
                [ 113,  119,  116,  115,  115,   85,  103,   89],
                [ 108,   95,  106,   97,   86,   89,   76,   88],
                [ 100,  101,  104,   99,   98,   92,   88,   90],
                [  89,  100,  101,   95,   93,   92,   89,   84],
                [  71,   82,   82,   77,   74,   79,   75,   65],
                [  70,   59,   70,   66,   68,   58,   55,   66],
                [  64,   69,   73,   72,   70,   79,   69,   53],
            ]),
            queen: PieceSquareTable([
                [  83,  106,  104,  110,  121,  112,  115,  103],
                [  70,   97,   95,  131,  161,  127,  166,  121],
                [  53,   55,   80,   97,  131,  143,  111,  135],
                [  37,   65,   64,  106,  128,  137,  127,  113],
                [  13,   47,   43,   89,   86,   84,   78,   95],
                [   2,   18,   43,   23,   25,   51,   32,   16],
                [  -7,    7,  -12,   11,    3,  -40,  -59,  -20],
                [  -2,  -18,  -18,   10,  -17,  -12,  -37,    7],
            ]),
            king: PieceSquareTable([
                [-168,  -69,  -40,  -22,  -36,  -30,  -41, -124],
                [ -48,   24,   19,   12,   13,   18,   37,  -24],
                [  -1,   33,   33,   27,   22,   38,   39,   -2],
                [   1,   32,   37,   46,   39,   35,   27,   -1],
                [ -22,   20,   34,   48,   45,   36,   18,  -14],
                [ -14,   17,   26,   41,   37,   30,   12,  -10],
                [ -18,    6,   20,   28,   19,   22,   -1,  -32],
                [ -57,  -32,  -19,  -18,  -54,  -11,  -36,  -85],
            ]),
        },
        mobility: Mobility {
            pawn: [-17, 9, 7, 17, 52],
            knight: [45, 78, 86, 90, 94, 96, 97, 97, 96],
            bishop: [19, 42, 56, 69, 83, 96, 106, 115, 120, 123, 122, 121, 122, 106],
            rook: [88, 102, 114, 121, 129, 135, 140, 140, 148, 155, 156, 162, 162, 158, 151],
            queen: [6, 28, 114, 145, 186, 202, 210, 226, 234, 242, 249, 255, 259, 268, 271, 275, 283, 283, 284, 290, 292, 290, 279, 278, 252, 260, 250, 261],
            king: [17, -7, -4, -3, -2, -2, -11, -11, -12],
        },
        passed_pawns: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [  23,   23,   15,   14,   10,   15,   22,   25],
            [ 124,  112,   94,   75,   47,   75,   84,  105],
            [  63,   53,   42,   46,   37,   39,   51,   49],
            [  29,   26,   22,   20,   17,   23,   40,   32],
            [   3,    4,    0,    3,   -2,   -2,   13,    4],
            [   9,    4,    8,   12,   -2,   -7,   -2,    9],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        bishop_pair: 71,
        doubled_pawns: -28,
    },
};
