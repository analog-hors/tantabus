use cozy_chess::*;

pub struct HistoryTable([[[i32; Square::NUM]; Square::NUM]; Color::NUM]);

impl HistoryTable {
    pub fn new() -> Self {
        Self([[[0; Square::NUM]; Square::NUM]; Color::NUM])
    }

    pub fn get(&self, color: Color, mv: Move) -> i32 {
        self.0
            [color as usize]
            [mv.from as usize]
            [mv.to as usize]
    }

    pub fn update(&mut self, color: Color, mv: Move, depth: u8, cutoff: bool) {
        let history = &mut self.0
            [color as usize]
            [mv.from as usize]
            [mv.to as usize];
        let change = depth as i32 * depth as i32;
        let decay = change * *history / 512;
        if cutoff {
            *history += change;
        } else {
            *history -= change;
        }
        *history -= decay;
        *history = (*history).clamp(-512, 512);
    }
}
