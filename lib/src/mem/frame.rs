//! Physical frame allocation.

use super::PAGE_SIZE;
use core::{ops::Range, slice};

const BITS_PER_WORD: usize = usize::BITS as usize;

/// Bitmap allocator for the frames between the lowest and highest RAM address.
///
/// A set bit marks a frame that is in use or isn't RAM.
pub(super) struct FrameAlloc {
    /// Frame number of bit 0.
    base: usize,
    bitmap: &'static mut [usize],
    /// No frame below this bit is free.
    hint: usize,
}

impl FrameAlloc {
    /// Returns the bytes of bitmap needed to cover the addresses in `span`.
    pub(super) const fn bitmap_size(span: &Range<usize>) -> usize {
        let frames = (span.end - span.start) / PAGE_SIZE;
        frames.div_ceil(BITS_PER_WORD) * size_of::<usize>()
    }

    /// Creates an allocator for the addresses in `span`, with every frame marked used.
    ///
    /// # Safety
    /// `span` must be page aligned. `storage` must be the address of
    /// [`bitmap_size`](Self::bitmap_size) bytes of writable, word aligned memory that nothing else
    /// uses from now on.
    pub(super) unsafe fn new(span: &Range<usize>, storage: usize) -> Self {
        let words = Self::bitmap_size(span) / size_of::<usize>();
        // SAFETY: The caller hands over `storage`, which is big enough for `words` words.
        let bitmap = unsafe {
            let ptr = storage as *mut usize;
            ptr.write_bytes(0xFF, words);
            slice::from_raw_parts_mut(ptr, words)
        };

        Self {
            base: span.start / PAGE_SIZE,
            bitmap,
            hint: 0,
        }
    }

    /// Marks every frame that lies fully or partly in `range` as free.
    pub(super) fn release(&mut self, range: Range<usize>) {
        for bit in self.bits(range) {
            self.bitmap[bit / BITS_PER_WORD] &= !(1 << (bit % BITS_PER_WORD));
        }
        self.hint = 0;
    }

    /// Marks every frame that lies fully or partly in `range` as used.
    ///
    /// Addresses the allocator doesn't cover are skipped.
    pub(super) fn reserve(&mut self, range: Range<usize>) {
        for bit in self.bits(range) {
            self.bitmap[bit / BITS_PER_WORD] |= 1 << (bit % BITS_PER_WORD);
        }
    }

    /// Allocates `count` contiguous frames, aligned to `align` frames, and returns their address.
    ///
    /// `align` must be a power of two.
    pub(super) fn alloc(&mut self, count: usize, align: usize) -> Option<usize> {
        let total = self.bitmap.len() * BITS_PER_WORD;
        // Alignment applies to the frame number, and `base` isn't necessarily aligned.
        let base = self.base;
        let align_up = |bit: usize| (base + bit).next_multiple_of(align) - base;

        let mut start = align_up(self.hint);
        while start.checked_add(count)? <= total {
            if let Some(used) = (start..start + count).find(|&bit| self.is_used(bit)) {
                start = align_up(used + 1);
                continue;
            }

            for bit in start..start + count {
                self.bitmap[bit / BITS_PER_WORD] |= 1 << (bit % BITS_PER_WORD);
            }
            // Only holds if alignment didn't skip any frames.
            if start == self.hint {
                self.hint = start + count;
            }
            return Some((base + start) * PAGE_SIZE);
        }

        None
    }

    /// Frees `count` frames starting at `addr`.
    ///
    /// # Panics
    /// Panics if any of the frames is already free.
    pub(super) fn free(&mut self, addr: usize, count: usize) {
        let start = addr / PAGE_SIZE - self.base;
        for bit in start..start + count {
            assert!(self.is_used(bit), "double free of frame at {addr:#x}");
            self.bitmap[bit / BITS_PER_WORD] &= !(1 << (bit % BITS_PER_WORD));
        }
        self.hint = self.hint.min(start);
    }

    fn is_used(&self, bit: usize) -> bool {
        self.bitmap[bit / BITS_PER_WORD] & (1 << (bit % BITS_PER_WORD)) != 0
    }

    /// Returns the bits of the frames touched by `range`, limited to the frames covered.
    fn bits(&self, range: Range<usize>) -> Range<usize> {
        let total = self.bitmap.len() * BITS_PER_WORD;
        let start = (range.start / PAGE_SIZE).saturating_sub(self.base);
        let end = range.end.div_ceil(PAGE_SIZE).saturating_sub(self.base);
        start.min(total)..end.min(total)
    }
}
