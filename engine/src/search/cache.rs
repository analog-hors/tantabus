use std::num::NonZeroU32;
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

pub type TableKeyValueEntry = Option<(u64, TableEntry)>;

#[derive(Debug)]
pub struct CacheTable {
    table: Box<[TableKeyValueEntry]>,
    len: u32
}

#[derive(Debug)]
pub enum CacheTableError {
    NotEnoughMemory,
    TooManyEntries
}

impl CacheTable {
    ///Create a cache table with a given number of entries.
    pub fn new_with_entries(entries: NonZeroU32) -> Self {
        Self {
            table: vec![None; entries.get() as usize].into_boxed_slice(),
            len: 0
        }
    }

    /// Create a cache table no bigger than a given size in bytes.
    /// # Errors
    /// There must be enough space for one [`TableKeyValueEntry`].
    /// If not, this will error with [`CacheTableError::NotEnoughMemory`].
    /// There must be at most [`u32::MAX`] entries.
    /// If not, this will error with [`CacheTableError::TooManyEntries`].
    pub fn new_with_size(size: usize) -> Result<Self, CacheTableError> {
        let entries = size / std::mem::size_of::<TableKeyValueEntry>();
        let entries: u32 = entries.try_into()
            .map_err(|_| CacheTableError::TooManyEntries)?;
        let entries = entries.try_into()
            .map_err(|_| CacheTableError::NotEnoughMemory)?;
        Ok(Self::new_with_entries(entries))
    }

    fn hash_to_index(&self, hash: u64) -> usize {
        ((hash as u32 as u64 * self.capacity() as u64) >> u32::BITS) as usize
    }

    pub fn get(&self, board: &Board) -> Option<TableEntry> {
        let hash = board.hash();
        let index = self.hash_to_index(hash);
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
        let index = self.hash_to_index(hash);
        let old = &mut self.table[index];
        if old.is_none() {
            self.len += 1;
        }
        *old = Some((hash, entry));
    }

    pub fn capacity(&self) -> u32 {
        self.table.len() as u32
    }

    pub fn len(&self) -> u32 {
        self.len
    }
}
