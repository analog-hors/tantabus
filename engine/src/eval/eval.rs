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
    mobility: PieceEvalSet<i16>
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
                [ 120,  121,  110,  119,  104,   83,    5,   -4],
                [ -59,  -17,    1,   12,   17,   27,   -7,  -42],
                [ -54,  -33,  -36,  -22,  -13,  -14,  -25,  -50],
                [ -56,  -37,  -38,  -34,  -26,  -29,  -20,  -44],
                [ -53,  -42,  -41,  -40,  -28,  -28,  -10,  -34],
                [ -61,  -51,  -55,  -54,  -54,  -27,  -12,  -50],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [-141,  -24,  -43,   -4,    4,  -27,   -4,  -51],
                [ -20,  -23,   18,   31,   34,   60,  -24,   -7],
                [ -34,    4,   38,   51,   84,   83,   39,   21],
                [  -9,    5,   26,   34,   19,   43,   14,   16],
                [ -15,   -5,   11,    7,   17,   13,   28,   -2],
                [ -23,   -5,    2,    7,   14,   11,    7,  -11],
                [ -53,  -38,  -21,  -10,  -15,  -11,  -37,  -30],
                [ -64,  -29,  -55,  -35,  -26,  -28,  -26,  -93],
            ]),
            bishop: PieceSquareTable([
                [ -34,  -18,  -45,  -34,  -37,  -47,    0,  -14],
                [ -44,  -13,  -12,  -17,   -7,   12,  -10,   -7],
                [ -19,   -5,   23,   14,   35,   37,   34,    7],
                [ -26,    1,    2,   31,    9,   18,   -2,   -9],
                [ -14,   -1,   -4,    5,   15,   -7,    0,    1],
                [  -7,    3,   -3,   -3,   -3,    3,    6,    6],
                [  -3,    0,    0,  -14,  -10,   -7,    6,   -4],
                [  -2,   -1,  -16,  -32,  -33,  -21,  -19,  -13],
            ]),
            rook: PieceSquareTable([
                [  -7,   -7,  -16,   -8,    1,   20,   18,   24],
                [ -44,  -50,  -24,    0,   -4,   22,   -7,   12],
                [ -75,  -43,  -41,  -17,    5,   16,   39,  -10],
                [ -78,  -68,  -55,  -39,  -53,  -41,  -30,  -40],
                [ -90,  -91,  -85,  -81,  -84,  -72,  -52,  -61],
                [ -92,  -83,  -88,  -81,  -83,  -78,  -40,  -67],
                [-113,  -81,  -77,  -81,  -81,  -68,  -59, -106],
                [ -76,  -71,  -65,  -60,  -64,  -69,  -48,  -69],
            ]),
            queen: PieceSquareTable([
                [  35,   30,   27,   51,   55,   74,   72,  105],
                [  38,   12,   61,   21,   63,  117,   77,  113],
                [  40,   50,   39,   72,   88,  124,  123,  124],
                [  67,   54,   60,   66,   64,  100,  108,  109],
                [  58,   64,   73,   63,   63,   87,   86,   93],
                [  43,   68,   73,   58,   53,   70,   82,   77],
                [  28,   56,   64,   58,   60,   59,   52,   42],
                [  37,   31,   52,   64,   45,  -16,    2,   -3],
            ]),
            king: PieceSquareTable([
                [  -1,    0,    1,    0,    0,    0,    0,   -3],
                [  -1,    8,   14,    5,    5,   11,    8,   -4],
                [   0,   19,   25,   10,   16,   30,   22,   -8],
                [  -1,   20,   21,    9,    5,   22,    9,  -27],
                [   0,   17,   29,   -8,   12,    9,   28,  -40],
                [ -13,   -6,    0,  -28,  -11,  -10,   14,  -16],
                [  19,   13,  -12,  -62,  -32,  -39,   19,   23],
                [ -14,   42,    7,  -75,  -13,  -70,   29,   28],
            ]),
        },
        mobility: PieceEvalSet {
            pawn: 4,
            knight: -1,
            bishop: 2,
            rook: 2,
            queen: 0,
            king: -6
        }
    },
    endgame: PhasedEval {
        piece_tables: PieceEvalSet {
            pawn: PieceSquareTable([
                [   0,    0,    0,    0,    0,    0,    0,    0],
                [ 156,  156,  147,  118,  115,  121,  156,  155],
                [ 130,  122,  104,   85,   75,   60,   92,   92],
                [  58,   48,   39,   24,   16,   17,   31,   31],
                [  37,   34,   16,   16,   12,   13,   20,   14],
                [  27,   27,   15,   18,   19,   16,   18,    5],
                [  25,   20,   19,   22,   24,   18,   10,   -9],
                [   0,    0,    0,    0,    0,    0,    0,    0],
            ]),
            knight: PieceSquareTable([
                [ -82,  -30,    0,  -12,   -6,   -5,  -20,  -56],
                [ -40,  -11,  -13,   17,    7,  -24,  -15,  -35],
                [ -21,   -6,    2,    6,   -7,   -4,  -10,  -28],
                [ -16,    0,    9,   24,   23,   12,   10,  -11],
                [ -21,    3,   12,   17,   18,   18,    5,   -5],
                [ -47,  -18,  -18,    0,   -5,   -8,  -13,  -43],
                [ -35,  -15,  -27,  -15,  -10,  -34,  -15,  -38],
                [ -81,  -70,  -24,  -18,  -25,  -33,  -60,  -46],
            ]),
            bishop: PieceSquareTable([
                [   5,    5,    5,   14,    9,    7,    0,    5],
                [   0,    1,    0,    4,    2,   -3,   -5,  -14],
                [  -6,    2,  -13,   -4,  -13,   -7,   -9,  -12],
                [  -4,   -5,   -8,  -12,   -3,  -15,    1,   -6],
                [ -15,   -8,   -6,   -8,  -17,   -4,  -10,  -15],
                [ -15,  -10,  -10,  -12,  -10,  -11,  -12,  -10],
                [ -31,  -29,  -24,  -15,  -20,  -30,  -25,  -55],
                [ -28,  -18,  -30,  -16,  -10,  -24,  -14,  -18],
            ]),
            rook: PieceSquareTable([
                [  75,   78,   81,   77,   73,   76,   71,   74],
                [  83,   91,   88,   87,   85,   63,   72,   63],
                [  83,   75,   84,   71,   58,   66,   45,   64],
                [  76,   77,   80,   75,   78,   72,   64,   64],
                [  66,   78,   80,   75,   75,   74,   68,   59],
                [  48,   57,   59,   56,   58,   63,   46,   41],
                [  48,   39,   48,   52,   52,   42,   39,   55],
                [  44,   50,   56,   51,   52,   59,   43,   27],
            ]),
            queen: PieceSquareTable([
                [  54,   67,   78,   67,   85,   95,   85,   64],
                [  45,   62,   21,   79,   82,   69,   59,   46],
                [  13,    6,   24,   19,   42,   52,   47,   43],
                [ -22,    4,  -10,    8,   44,   29,   31,   25],
                [  -9,    2,  -30,   10,   14,   20,   19,   33],
                [  18,   -3,  -12,   -8,   10,   22,   10,    5],
                [  31,   -3,  -24,   -5,   -8,  -21,   -8,   11],
                [  23,   27,   -3,  -10,   13,   73,   12,   14],
            ]),
            king: PieceSquareTable([
                [ -33,  -15,  -11,   -4,  -12,   -7,   -3,  -34],
                [ -19,   31,   23,   10,   17,   25,   41,  -27],
                [   4,   37,   36,   27,   28,   40,   43,   -1],
                [   1,   25,   35,   37,   36,   34,   29,   -4],
                [ -32,    9,   22,   34,   30,   24,   10,  -15],
                [ -25,    2,    8,   21,   16,   14,   -3,  -22],
                [ -28,  -16,   -1,    8,    0,    4,  -20,  -47],
                [ -65,  -50,  -34,  -33,  -65,  -22,  -51, -104],
            ]),
        },
        mobility: PieceEvalSet {
            pawn: 14,
            knight: 5,
            bishop: 7,
            rook: 5,
            queen: 11,
            king: -1
        }
    }
};
