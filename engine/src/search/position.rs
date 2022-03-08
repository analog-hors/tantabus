use arrayvec::ArrayVec;
use cozy_chess::*;

use crate::eval::Eval;
use crate::nnue::*;

#[derive(Clone)]
pub struct Position<'s> {
    board: Board,
    nnue_state: NnueState<'s>
}

impl<'s> Position<'s> {
    pub fn new(model: &'s Nnue, board: Board) -> Self {
        let mut nnue_state = model.new_state();
        for &color in &Color::ALL {
            let colors = board.colors(color);
            for &piece in &Piece::ALL {
                let pieces = board.pieces(piece);
                for square in pieces & colors {
                    nnue_state.add(color, piece, square);
                }
            }
        }
        Self {
            board,
            nnue_state
        }
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn play_unchecked(&self, mv: Move) -> Self {
        let mut updates = ArrayVec::<_, 3>::new();
        let moved = self.board.piece_on(mv.from).unwrap();
        updates.push((self.board.color_on(mv.from).unwrap(), moved));
        if let Some(color) = self.board.color_on(mv.to) {
            updates.push((color, self.board.piece_on(mv.to).unwrap()));
        }
        if let Some(piece) = mv.promotion {
            updates.push((self.board.color_on(mv.from).unwrap(), piece));
        }
        if moved == Piece::Pawn {
            let ep_square = self.board.en_passant().map(|ep| {
                Square::new(ep, Rank::Third.relative_to(!self.board.side_to_move()))
            });
            if Some(mv.to) == ep_square {
                updates.push((!self.board.side_to_move(), Piece::Pawn));
            }
        }
        let mut new = self.clone();
        new.board.play_unchecked(mv);
        for &(color, piece) in &updates {
            let old_pieces = self.board.colors(color) & self.board.pieces(piece);
            let new_pieces = new.board.colors(color) & new.board.pieces(piece);
            for square in old_pieces & !new_pieces {
                new.nnue_state.sub(color, piece, square);
            }
            for square in new_pieces & !old_pieces {
                new.nnue_state.add(color, piece, square);
            }
        }
        // debug_assert_eq!(
        //     new.nnue_state.accumulator(),
        //     Position::new(new.nnue_state.model(), new.board.clone()).nnue_state.accumulator(),
        //     "{}\n{}\n{:?}",
        //     self.board, mv, updates
        // );
        new
    }

    pub fn null_move(&self) -> Option<Self> {
        Some(Self {
            board: self.board.null_move()?,
            nnue_state: self.nnue_state.clone()
        })
    }

    pub fn evaluate(&self) -> Eval {
        Eval::cp(self.nnue_state.evaluate(self.board.side_to_move()) as i16)
    }
}
