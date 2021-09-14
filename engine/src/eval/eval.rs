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
pub struct StandardEvaluator {
    pub piece_values: PieceEvalSet<i16>,
    pub midgame_piece_tables: PieceEvalSet<PieceSquareTable>,
    pub endgame_piece_tables: PieceEvalSet<PieceSquareTable>
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

    fn evaluate_for_side(&self, board: &Board, side: Color, phase: u32) -> i16 {
        let mut value = 0;
        let mut midgame_value = 0;
        let mut endgame_value = 0;
        let ally_pieces = board.colors(side);

        for &piece in &Piece::ALL {
            let pieces = ally_pieces & board.pieces(piece);
            let piece_value = *self.piece_values.get(piece);
            let midgame_piece_table = self.midgame_piece_tables.get(piece);
            let endgame_piece_table = self.endgame_piece_tables.get(piece);

            value += pieces.popcnt() as i16 * piece_value;
            for square in pieces {
                midgame_value += midgame_piece_table.get(side, square);
                endgame_value += endgame_piece_table.get(side, square);
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
    midgame_piece_tables: PieceEvalSet {
        pawn: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [ 134,  126,  115,  121,  101,   79,   19,   11],
            [ -50,   -6,   14,   21,   29,   46,    5,  -32],
            [ -52,  -29,  -34,  -20,  -10,  -10,  -20,  -47],
            [ -58,  -38,  -42,  -37,  -30,  -35,  -21,  -46],
            [ -59,  -44,  -47,  -45,  -34,  -39,  -11,  -37],
            [ -63,  -50,  -56,  -54,  -56,  -27,   -4,  -41],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        knight: PieceSquareTable([
            [-112,  -19,  -31,   -5,    0,  -22,   -4,  -36],
            [ -25,  -28,   17,   25,   31,   54,  -22,  -14],
            [ -33,    4,   36,   48,   83,   83,   45,   21],
            [ -14,   -1,   19,   29,   11,   38,    8,   17],
            [ -27,  -19,   -3,   -8,    1,   -3,   15,  -15],
            [ -42,  -23,  -17,  -10,   -4,  -13,  -13,  -30],
            [ -67,  -52,  -35,  -23,  -29,  -25,  -47,  -33],
            [ -55,  -44,  -77,  -56,  -46,  -38,  -41,  -82],
        ]),
        bishop: PieceSquareTable([
            [ -22,  -10,  -30,  -20,  -25,  -31,    1,  -10],
            [ -32,   13,    5,   -3,   10,   23,    4,    4],
            [  -2,   14,   52,   42,   58,   65,   57,   30],
            [ -14,   20,   23,   57,   34,   48,   15,    8],
            [ -11,    9,    9,   18,   31,    4,    8,    5],
            [  -5,    4,    2,    8,    2,    3,    4,   10],
            [   2,    3,    9,   -9,   -7,   -1,   16,    8],
            [  -2,    0,  -22,  -37,  -39,  -21,  -17,   -9],
        ]),
        rook: PieceSquareTable([
            [  11,   11,    6,   11,   15,   25,   21,   32],
            [ -15,  -17,    9,   20,   17,   37,   10,   32],
            [ -48,  -11,   -5,   12,   35,   42,   56,   21],
            [ -59,  -43,  -28,   -7,  -21,  -15,   -1,   -8],
            [ -81,  -81,  -73,  -67,  -72,  -63,  -38,  -44],
            [ -93,  -79,  -85,  -82,  -84,  -83,  -33,  -62],
            [-119,  -79,  -75,  -83,  -84,  -65,  -49,  -96],
            [ -75,  -69,  -66,  -64,  -67,  -69,  -39,  -67],
        ]),
        queen: PieceSquareTable([
            [  24,   24,   32,   52,   54,   64,   53,   79],
            [  22,   -8,   51,   22,   59,  104,   65,   87],
            [  27,   38,   28,   61,   79,  123,  123,  124],
            [  47,   37,   42,   47,   48,   94,   96,  102],
            [  29,   38,   51,   39,   33,   62,   64,   73],
            [   8,   36,   45,   26,   16,   35,   48,   43],
            [ -12,   23,   34,   25,   26,   25,   23,   15],
            [  -5,  -27,   -3,   29,  -12,  -56,  -22,  -23],
        ]),
        king: PieceSquareTable([
            [  -1,    0,    1,    0,    0,    0,    0,   -2],
            [  -1,    6,   10,    3,    4,    8,    6,   -4],
            [   0,   15,   19,    8,   12,   23,   18,   -5],
            [   0,   15,   16,    7,    4,   17,    7,  -19],
            [  -1,   11,   23,   -6,   13,    9,   23,  -33],
            [ -12,  -11,   -4,  -26,   -8,  -14,   17,  -11],
            [  25,    8,  -13,  -67,  -34,  -41,   22,   40],
            [  -4,   50,   11,  -80,  -12,  -74,   37,   46],
        ]),
    },
    endgame_piece_tables: PieceEvalSet {
        pawn: PieceSquareTable([
            [   0,    0,    0,    0,    0,    0,    0,    0],
            [ 168,  163,  155,  129,  125,  127,  156,  156],
            [ 149,  138,  117,   98,   86,   77,  105,  108],
            [  75,   65,   54,   34,   29,   32,   47,   46],
            [  52,   48,   27,   28,   24,   26,   33,   29],
            [  44,   42,   30,   32,   36,   30,   34,   22],
            [  55,   49,   44,   43,   47,   46,   39,   20],
            [   0,    0,    0,    0,    0,    0,    0,    0],
        ]),
        knight: PieceSquareTable([
            [ -76,  -34,  -12,  -21,  -17,  -15,  -21,  -43],
            [ -61,  -28,  -18,   14,    5,  -26,  -31,  -52],
            [ -39,   -7,   11,   14,   -2,    9,  -11,  -41],
            [ -33,    2,   16,   32,   36,   24,   18,  -25],
            [ -43,   -4,   17,   26,   22,   22,   -5,  -26],
            [ -77,  -30,  -18,    2,   -2,  -20,  -31,  -76],
            [ -56,  -36,  -41,  -30,  -26,  -48,  -42,  -71],
            [ -77, -112,  -50,  -47,  -51,  -65,  -94,  -48],
        ]),
        bishop: PieceSquareTable([
            [   5,    8,    8,   18,   14,    9,    1,    6],
            [  -5,   15,   13,   19,   18,   12,   10,  -12],
            [   0,   17,    9,   16,    9,   25,   10,    0],
            [   1,   16,   13,   19,   32,   10,   28,    3],
            [ -12,    9,   21,   22,   16,   18,    1,  -14],
            [ -13,    1,   13,   13,   12,    3,  -12,  -13],
            [ -28,  -19,  -14,   -5,   -8,  -25,  -15,  -61],
            [ -30,  -20,  -42,  -20,  -16,  -31,  -21,  -24],
        ]),
        rook: PieceSquareTable([
            [  78,   84,   88,   84,   82,   81,   76,   76],
            [  79,   87,   85,   90,   84,   63,   67,   58],
            [  78,   71,   78,   68,   52,   66,   43,   54],
            [  70,   73,   75,   67,   70,   64,   52,   51],
            [  54,   70,   73,   69,   67,   61,   50,   41],
            [  37,   47,   49,   50,   50,   49,   28,   26],
            [  41,   31,   37,   44,   42,   33,   25,   43],
            [  34,   43,   53,   51,   49,   50,   38,   15],
        ]),
        queen: PieceSquareTable([
            [  46,   62,   77,   75,   84,   84,   68,   54],
            [  29,   72,   37,   81,   92,   73,   47,   36],
            [   7,   15,   46,   46,   63,   70,   42,   17],
            [ -22,   21,   18,   51,   83,   48,   37,    9],
            [ -13,   15,   -7,   45,   51,   29,    7,    7],
            [   6,  -12,   -3,   13,   26,   17,   -8,  -11],
            [  15,   -7,  -31,   -7,  -13,  -28,  -19,    0],
            [  20,   33,    7,  -39,   21,   45,   -3,    1],
        ]),
        king: PieceSquareTable([
            [ -23,   -9,   -6,   -1,   -7,   -4,   -1,  -23],
            [ -14,   30,   22,    9,   16,   23,   40,  -23],
            [   9,   39,   37,   27,   28,   45,   47,    4],
            [   5,   27,   37,   37,   37,   38,   33,    0],
            [ -32,    7,   22,   34,   30,   22,    6,  -16],
            [ -27,   -4,    3,   19,   14,    5,  -14,  -26],
            [ -25,  -20,   -6,    3,   -4,    0,  -25,  -47],
            [ -56,  -48,  -33,  -36,  -71,  -21,  -47,  -99],
        ]),
    },
};
