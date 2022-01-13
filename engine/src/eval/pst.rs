use cozy_chess::*;
use serde::{Serialize, Deserialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct KingRelativePst(pub [[[i16; 4]; 8]; 2]);

impl KingRelativePst {
    fn key(side: Color, king: Square, square: Square) -> (usize, usize, usize) {
        let on_king_half = (king.file() > File::D) == (square.file() > File::D);
        let rank = square.rank().relative_to(!side);
        let file = if square.file() > File::D {
            square.file().flip()
        } else {
            square.file()
        };
        (on_king_half as usize, rank as usize, file as usize)
    }

    pub fn get(&self, side: Color, king: Square, square: Square) -> i16 {
        let (on_king_half, rank, file) = Self::key(side, king, square);
        self.0[on_king_half][rank][file]
    }

    pub fn get_mut(&mut self, side: Color, king: Square, square: Square) -> &mut i16 {
        let (on_king_half, rank, file) = Self::key(side, king, square);
        &mut self.0[on_king_half][rank][file]
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pst(pub [[i16; 8]; 8]);

impl Pst {
    fn key(side: Color, square: Square) -> (usize, usize) {
        let rank = square.rank().relative_to(!side);
        (rank as usize, square.file() as usize)
    }
    
    pub fn get(&self, side: Color, square: Square) -> i16 {
        let (rank, file) = Self::key(side, square);
        self.0[rank][file]
    }

    pub fn get_mut(&mut self, side: Color, square: Square) -> &mut i16 {
        let (rank, file) = Self::key(side, square);
        &mut self.0[rank][file]
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PstEvalSet {
    pub pawn: KingRelativePst,
    pub knight: KingRelativePst,
    pub bishop: KingRelativePst,
    pub rook: KingRelativePst,
    pub queen: KingRelativePst,
    pub king: Pst
}

impl PstEvalSet {
    pub fn get(&self, piece: Piece, color: Color, king: Square, square: Square) -> i16 {
        if piece == Piece::King {
            self.king.get(color, square)
        } else {
            let table = match piece {
                Piece::Pawn => &self.pawn,
                Piece::Knight => &self.knight,
                Piece::Bishop => &self.bishop,
                Piece::Rook => &self.rook,
                Piece::Queen => &self.queen,
                Piece::King => unreachable!()
            };
            table.get(color, king, square)
        }
    }

    pub fn get_mut(&mut self, piece: Piece, color: Color, king: Square, square: Square) -> &mut i16 {
        if piece == Piece::King {
            self.king.get_mut(color, square)
        } else {
            let table = match piece {
                Piece::Pawn => &mut self.pawn,
                Piece::Knight => &mut self.knight,
                Piece::Bishop => &mut self.bishop,
                Piece::Rook => &mut self.rook,
                Piece::Queen => &mut self.queen,
                Piece::King => unreachable!()
            };
            table.get_mut(color, king, square)
        }
    }
}
