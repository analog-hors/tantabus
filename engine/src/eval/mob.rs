use cozy_chess::*;
use serde::{Serialize, Deserialize};

// CITE: Mobility evaluation.
// https://www.chessprogramming.org/Mobility
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
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

    pub fn get_mut(&mut self, piece: Piece) -> &mut [i16] {
        match piece {
            Piece::Pawn => &mut self.pawn,
            Piece::Knight => &mut self.knight,
            Piece::Bishop => &mut self.bishop,
            Piece::Rook => &mut self.rook,
            Piece::Queen => &mut self.queen,
            Piece::King => &mut self.king
        }
    }
}
