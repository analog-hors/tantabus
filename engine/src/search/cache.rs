use std::num::NonZeroU32;
use std::sync::atomic::{Ordering, AtomicU64};
use bytemuck::{Pod, Zeroable};
use cozy_chess::*;

use crate::eval::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CacheDataKind {
    Exact,
    LowerBound,
    UpperBound
}

#[derive(Debug, Copy, Clone)]
pub struct CacheData {
    pub kind: CacheDataKind,
    pub eval: Eval,
    pub depth: u8,
    pub best_move: Move
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct EncodedEntry {
    kind: u8,
    eval: [u8; 2],
    depth: u8,
    best_move_from: u8,
    best_move_to: u8,
    best_move_promotion: u8,
    _padding: u8
}

#[derive(Debug)]
struct CacheEntry {
    hash_xor_data: AtomicU64,
    data: AtomicU64
}

impl CacheEntry {
    fn empty() -> Self {
        Self {
            hash_xor_data: AtomicU64::new(0),
            data: AtomicU64::new(0)
        }
    }

    fn is_empty(&self) -> bool {
        self.data.load(Ordering::Acquire) == 0
    }

    fn store(&self, hash: u64, entry: CacheData) {
        let data = bytemuck::cast(EncodedEntry {
            kind: entry.kind as u8,
            eval: entry.eval.to_bytes(),
            depth: entry.depth,
            best_move_from: entry.best_move.from as u8,
            best_move_to: entry.best_move.to as u8,
            best_move_promotion: entry.best_move.promotion.map_or(u8::MAX, |p| p as u8),
            _padding: 0,
        });
        self.hash_xor_data.store(hash ^ data, Ordering::Relaxed);
        self.data.store(data, Ordering::Relaxed);
    }

    fn load(&self, hash: u64) -> Option<CacheData> {
        let hash_xor_data = self.hash_xor_data.load(Ordering::Relaxed);
        let data = self.data.load(Ordering::Relaxed);
        if data == 0 || hash_xor_data ^ data != hash {
            return None;
        }
        let data: EncodedEntry = bytemuck::cast(data);
        Some(CacheData {
            kind: match data.kind {
                0 => CacheDataKind::Exact,
                1 => CacheDataKind::LowerBound,
                2 => CacheDataKind::UpperBound,
                _ => unreachable!()
            },
            eval: Eval::from_bytes(data.eval),
            depth: data.depth,
            best_move: Move {
                from: Square::index(data.best_move_from as usize),
                to: Square::index(data.best_move_to as usize),
                promotion: Piece::try_index(data.best_move_promotion as usize)
            },
        })
    }
}

// CITE: Transposition table.
// https://www.chessprogramming.org/Transposition_Table
#[derive(Debug)]
pub struct CacheTable {
    table: Box<[CacheEntry]>,
}

#[derive(Debug)]
pub enum CacheTableError {
    NotEnoughMemory,
    TooManyEntries
}

#[test]
fn print() {
    panic!("{}", std::mem::size_of::<CacheEntry>());
}

impl CacheTable {
    /// Create a cache table with a given number of entries.
    pub fn new_with_entries(entries: NonZeroU32) -> Self {
        Self {
            table: (0..entries.get()).map(|_| CacheEntry::empty()).collect()
        }
    }

    /// Create a cache table no bigger than a given size in bytes.
    /// # Errors
    /// There must be enough space for one [`TableKeyValueEntry`].
    /// If not, this will error with [`CacheTableError::NotEnoughMemory`].
    /// There must be at most [`u32::MAX`] entries.
    /// If not, this will error with [`CacheTableError::TooManyEntries`].
    pub fn new_with_size(size: usize) -> Result<Self, CacheTableError> {
        let entries = size / std::mem::size_of::<CacheEntry>();
        let entries: u32 = entries.try_into()
            .map_err(|_| CacheTableError::TooManyEntries)?;
        let entries = entries.try_into()
            .map_err(|_| CacheTableError::NotEnoughMemory)?;
        Ok(Self::new_with_entries(entries))
    }

    fn hash_to_index(&self, hash: u64) -> usize {
        // CITE: This reduction scheme was first observed in Stockfish,
        // who implemented it after a blog post by Daniel Lemire.
        // https://github.com/official-stockfish/Stockfish/commit/2198cd0524574f0d9df8c0ec9aaf14ad8c94402b
        // https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
        ((hash as u32 as u64 * self.capacity() as u64) >> u32::BITS) as usize
    }

    pub fn prefetch(&self, board: &Board) {
        let index = self.hash_to_index(board.hash());
        let entry = &self.table[index];
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
            _mm_prefetch(entry as *const _ as *const _, _MM_HINT_T0);
        }
        let _ = entry;
    }

    pub fn get(&self, board: &Board, ply_index: u8) -> Option<CacheData> {
        let hash = board.hash();
        let index = self.hash_to_index(hash);
        if let Some(mut data) = self.table[index].load(hash) {
            data.eval = match data.eval.kind() {
                EvalKind::Centipawn(_) => data.eval,
                // Mate scores can sometimes get really big.
                // I'm not sure why this happens.
                // Ethereal seems to have had a similar problem at some point.
                // It seems related to bad interactions with "unresolved" mates and TT grafting.
                // Scores seem to be stored as large, inexact bounds.
                // In any case, for now, this ignores it by turning it into a high eval instead of a mate score.
                EvalKind::MateIn(p) => {
                    let p = p as u32 + ply_index as u32;
                    if p <= u8::MAX as u32 {
                        Eval::mate_in(p as u8)
                    } else {
                        Eval::cp((20000 - p - u8::MAX as u32) as i16)
                    }
                },
                EvalKind::MatedIn(p) => {
                    let p = p as u32 + ply_index as u32;
                    if p <= u8::MAX as u32 {
                        Eval::mated_in(p as u8)
                    } else {
                        Eval::cp(-((20000 - p - u8::MAX as u32) as i16))
                    }
                },
            };
            return Some(data);
        }
        None
    }

    pub fn set(&self, board: &Board, ply_index: u8, mut entry: CacheData) {
        entry.eval = match entry.eval.kind() {
            EvalKind::Centipawn(_) => entry.eval,
            EvalKind::MateIn(p) => Eval::mate_in(p - ply_index),
            EvalKind::MatedIn(p) => Eval::mated_in(p - ply_index),
        };
        let hash = board.hash();
        let index = self.hash_to_index(hash);
        self.table[index].store(hash, entry);
    }

    pub fn capacity(&self) -> u32 {
        self.table.len() as u32
    }

    pub fn approx_size_permill(&self) -> u32 {
        self.table.iter().take(1000).filter(|e| !e.is_empty()).count() as u32
    }

    pub fn clear(&mut self) {
        for entry in self.table.iter_mut() {
            *entry = CacheEntry::empty();
        }
    }
}
