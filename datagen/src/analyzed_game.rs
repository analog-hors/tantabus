use std::io::prelude::*;

use cozy_chess::*;
use tantabus::eval::Eval;

pub struct AnalyzedGame {
    pub opening_moves: u8,
    pub moves: Vec<Move>,
    pub evals: Vec<Eval>,
    pub winner: Option<Color>
}

fn pack_move(mv: Move) -> u16 {
    let mut packed = 0;
    packed = (packed << 6) | mv.from as u16;
    packed = (packed << 6) | mv.to as u16;
    packed = (packed << 4) | mv.promotion.map_or(0b1111, |p| p as u16);
    packed
}

fn unpack_move(mut packed: u16) -> Move {
    let promotion = packed & 0b1111;
    packed >>= 4;
    let to = packed & 0b111111;
    packed >>= 6;
    let from = packed & 0b111111;

    let promotion = if promotion != 0b1111 {
        Some(Piece::index(promotion as usize))
    } else {
        None
    };
    let to = Square::index(to as usize);
    let from = Square::index(from as usize);
    Move {
        from,
        to,
        promotion
    }
}

pub fn write_analyzed_game(analysis: &AnalyzedGame, out: &mut impl Write) -> std::io::Result<()> {
    assert_eq!(analysis.moves.len() - analysis.opening_moves as usize, analysis.evals.len());
    out.write_all(&[analysis.opening_moves])?;
    out.write_all(&(analysis.moves.len() as u16).to_le_bytes())?;
    for &mv in &analysis.moves {
        out.write_all(&pack_move(mv).to_le_bytes())?;
    }
    for &eval in &analysis.evals {
        out.write_all(&eval.to_bytes())?;
    }
    out.write_all(&[analysis.winner.map_or(2, |c| c as u8)])?;
    Ok(())
}

pub fn read_analyzed_game(reader: &mut impl Read) -> std::io::Result<Option<AnalyzedGame>> {
    let mut started_reading = false;
    let result = (|| {
        macro_rules! read_num {
            ($type:ty) => {{
                let mut buffer = <$type>::to_le_bytes(0);
                reader.read_exact(&mut buffer)?;
                <$type>::from_le_bytes(buffer)
            }}
        }
    
        let opening_moves = read_num!(u8);
        started_reading = true;
    
        let moves_len = read_num!(u16) as usize;
        let mut moves = Vec::with_capacity(moves_len);
        for _ in 0..moves_len {
            moves.push(unpack_move(read_num!(u16)));
        }
    
        let evals_len = moves_len - opening_moves as usize;
        let mut evals = Vec::with_capacity(evals_len);
        for _ in 0..evals_len {
            let mut eval = [0; 2];
            reader.read_exact(&mut eval)?;
            evals.push(Eval::from_bytes(eval));
        }
    
        let winner = read_num!(u8);
        let winner = if winner != 2 {
            Some(Color::index(winner as usize))
        } else {
            None
        };
        
        Ok(AnalyzedGame {
            opening_moves,
            moves,
            evals,
            winner,
        })
    })();

    if !started_reading {
        return Ok(None);
    }
    
    result.map(Some)
}
