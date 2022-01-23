use cozy_chess::*;
use serde::{Serialize, Deserialize};

use super::Eval;
use super::pst::*;
use super::mob::*;
use super::trace::*;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EvalTerms {
    pub piece_tables: PstEvalSet,
    pub mobility: Mobility,
    pub passed_pawns: KingRelativePst,
    pub bishop_pair: i16,
    pub rook_on_open_file: i16,
    pub rook_on_semiopen_file: i16
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Evaluator {
    pub midgame: EvalTerms,
    pub endgame: EvalTerms
}

struct EvalContext<'c, T> {
    board: &'c Board,
    color: Color,
    mg: &'c mut i16,
    eg: &'c mut i16,
    trace: &'c mut T
}

impl Evaluator {
    pub fn evaluate(&self, board: &Board) -> Eval {
        let phase = Self::game_phase(board);
        let us = self.evaluate_for_side(board, board.side_to_move(), phase, &mut ());
        let them = self.evaluate_for_side(board, !board.side_to_move(), phase, &mut ());
        Eval::cp(us - them)
    }

    pub fn eval_trace(&self, board: &Board) -> (EvalTerms, EvalTerms, u32) {
        let mut our_features = EvalTerms::default();
        let mut their_features = EvalTerms::default();
        let phase = Self::game_phase(board);
        self.evaluate_for_side(board, board.side_to_move(), phase, &mut our_features);
        self.evaluate_for_side(board, !board.side_to_move(), phase, &mut their_features);
        (our_features, their_features, phase)
    }

    pub const MAX_PHASE: u32 = 256;

    // CITE: This way of calculating the game phase was apparently done in Fruit.
    // https://www.chessprogramming.org/Tapered_Eval#Implementation_example
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

    fn evaluate_for_side(&self, board: &Board, color: Color, phase: u32, trace: &mut impl TraceTarget) -> i16 {
        let mut midgame_value = 0;
        let mut endgame_value = 0;
        let mut ctx = EvalContext {
            board,
            color,
            mg: &mut midgame_value,
            eg: &mut endgame_value,
            trace,
        };
        self.add_psqt_terms(&mut ctx);
        self.add_mobility_terms(&mut ctx);
        self.add_passed_pawn_terms(&mut ctx);
        self.add_rook_on_open_file_terms(&mut ctx);
        self.add_bishop_pair_terms(&mut ctx);

        let phase = phase as i32;
        const MAX_PHASE: i32 = Evaluator::MAX_PHASE as i32;
        let interpolated = (
            (midgame_value as i32 * (MAX_PHASE - phase)) +
            (endgame_value as i32 * phase)
        ) / MAX_PHASE;
        interpolated as i16
    }

    fn add_psqt_terms<T: TraceTarget>(&self, ctx: &mut EvalContext<T>) {
        let our_pieces = ctx.board.colors(ctx.color);
        let our_king = ctx.board.king(ctx.color);
        for &piece in &Piece::ALL {
            let pieces = our_pieces & ctx.board.pieces(piece);
            for square in pieces {
                ctx.trace.trace(|terms| {
                    *terms.piece_tables.get_mut(piece, ctx.color, our_king, square) += 1;
                });
                *ctx.mg += self.midgame.piece_tables.get(piece, ctx.color, our_king, square);
                *ctx.eg += self.endgame.piece_tables.get(piece, ctx.color, our_king, square);
            }
        }
    }

    fn add_mobility_terms<T: TraceTarget>(&self, ctx: &mut EvalContext<T>) {
        let our_pieces = ctx.board.colors(ctx.color);
        let occupied = ctx.board.occupied();
        for &piece in &Piece::ALL {
            let pieces = our_pieces & ctx.board.pieces(piece);
            let midgame_mobility = self.midgame.mobility.get(piece);
            let endgame_mobility = self.endgame.mobility.get(piece);

            for square in pieces {
                let approx_moves = match piece {
                    Piece::Pawn => (
                        get_pawn_quiets(square, ctx.color, occupied) |
                        (get_pawn_attacks(square, ctx.color) & ctx.board.colors(!ctx.color))
                    ),
                    Piece::Knight => get_knight_moves(square) & !our_pieces,
                    Piece::Bishop => get_bishop_moves(square, occupied) & !our_pieces,
                    Piece::Rook => get_rook_moves(square, occupied) & !our_pieces,
                    Piece::Queen => (
                        get_bishop_moves(square, occupied) |
                        get_rook_moves(square, occupied)
                    ) & !our_pieces,
                    Piece::King => get_king_moves(square) & !our_pieces
                };
                let mobility = approx_moves.popcnt() as usize;
                ctx.trace.trace(|terms| {
                    terms.mobility.get_mut(piece)[mobility] += 1;
                });
                *ctx.mg += midgame_mobility[mobility];
                *ctx.eg += endgame_mobility[mobility];
            }
        }
    }

    fn add_passed_pawn_terms<T: TraceTarget>(&self, ctx: &mut EvalContext<T>) {
        let our_pieces = ctx.board.colors(ctx.color);
        let pawns = ctx.board.pieces(Piece::Pawn);
        let our_pawns = our_pieces & pawns;
        let their_pawns = pawns ^ our_pawns;
        let our_king = ctx.board.king(ctx.color);
        let promotion_rank = Rank::Eighth.relative_to(ctx.color);

        for pawn in our_pawns {
            let telestop = Square::new(pawn.file(), promotion_rank);
            let front_span = get_between_rays(pawn, telestop);
            let mut blocker_mask = front_span;
            for attack in get_pawn_attacks(pawn, ctx.color) {
                let telestop = Square::new(attack.file(), promotion_rank);
                let front_span = get_between_rays(attack, telestop);
                blocker_mask |= front_span | attack.bitboard();
            }

            let passed = (their_pawns & blocker_mask).is_empty()
                && (our_pawns & front_span).is_empty();
            if passed {
                ctx.trace.trace(|terms| {
                    *terms.passed_pawns.get_mut(ctx.color, our_king, pawn) += 1;
                });
                *ctx.mg += self.midgame.passed_pawns.get(ctx.color, our_king, pawn);
                *ctx.eg += self.endgame.passed_pawns.get(ctx.color, our_king, pawn);
            }
        }
    }

    fn add_rook_on_open_file_terms<T: TraceTarget>(&self, ctx: &mut EvalContext<T>) {
        let our_pieces = ctx.board.colors(ctx.color);
        let pawns = ctx.board.pieces(Piece::Pawn);
        let our_pawns = our_pieces & pawns;
        let our_rooks = our_pieces & ctx.board.pieces(Piece::Rook);
        
        for rook in our_rooks {
            let file = rook.file();
            let file_bb = file.bitboard();
            if (file_bb & pawns).is_empty() {
                ctx.trace.trace(|terms| {
                    terms.rook_on_open_file += 1;
                });
                *ctx.mg += self.midgame.rook_on_open_file;
                *ctx.eg += self.endgame.rook_on_open_file;
            } else if (file_bb & our_pawns).is_empty() {
                ctx.trace.trace(|terms| {
                    terms.rook_on_semiopen_file += 1;
                });
                *ctx.mg += self.midgame.rook_on_semiopen_file;
                *ctx.eg += self.endgame.rook_on_semiopen_file;
            }
        }
    }

    fn add_bishop_pair_terms<T: TraceTarget>(&self, ctx: &mut EvalContext<T>) {
        let our_pieces = ctx.board.colors(ctx.color);
        if (our_pieces & ctx.board.pieces(Piece::Bishop)).popcnt() >= 2 {
            ctx.trace.trace(|terms| {
                terms.bishop_pair += 1;
            });
            *ctx.mg += self.midgame.bishop_pair;
            *ctx.eg += self.endgame.bishop_pair;
        }
    }
}
