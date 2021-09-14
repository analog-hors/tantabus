use cozy_chess::*;

use crate::eval::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TableEntryKind {
    Exact,
    LowerBound,
    UpperBound
}

#[derive(Debug, Copy, Clone)]
pub struct TableEntry {
    pub kind: TableEntryKind,
    pub eval: Eval,
    pub depth: u8,
    pub best_move: Move
}

type FullTableEntry = Option<(u64, TableEntry)>;

#[derive(Debug)]
pub struct CacheTable {
    table: Box<[FullTableEntry]>,
    len: usize,
    mask: usize
}

impl CacheTable {
    ///Rounds up the number of entries to a power of two.
    ///`panic` on overflow.
    pub fn with_rounded_entries(entries: usize) -> Self {
        let entries = entries.checked_next_power_of_two().unwrap();
        let table = vec![None; entries].into_boxed_slice();
        Self {
            len: 0,
            mask: table.len() - 1,
            table
        }
    }

    ///Converts the size in bytes to an amount of entries
    ///then rounds up the size to the nearest power of two.
    ///`panic` on overflow.
    pub fn with_rounded_size(size: usize) -> Self {
        Self::with_rounded_entries(size / std::mem::size_of::<FullTableEntry>())
    }

    pub fn get(&self, board: &Board) -> Option<TableEntry> {
        let hash = board.hash();
        let index = hash as usize & self.mask;
        if let Some((entry_hash, entry)) = self.table[index] {
            if entry_hash == hash {
                return Some(entry);
            }
        }
        None
    }

    pub fn set(
        &mut self,
        board: &Board,
        entry: TableEntry
    ) {
        let hash = board.hash();
        let index = hash as usize & self.mask;
        let old = &mut self.table[index];
        if let Some(old) = old {
            if old.0 == hash || entry.depth > old.1.depth {
                //Matching hashes uses the newer entry since it has more information.
                //Otherwise, select the deeper entry.
                *old = (hash, entry);
            }
        } else {
            //Insert to empty slot
            self.len += 1;
            *old = Some((hash, entry));
        }
    }

    pub fn capacity(&self) -> usize {
        self.table.len()
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
