use crate::syscalls;
use crate::{sync::*, syscalls::*};
use core::{
    alloc::{GlobalAlloc, Layout},
    isize,
    mem::{size_of, MaybeUninit},
    ptr::null_mut,
    usize,
};
// TODO: switch this to a linked list arena approach?

// adjustable contants
const BLOCK_SHIFT: usize = 14;
// Allow a maximum loss of 12.5%. For everything more, mmap
// Maximum loss is acchieved with minimum (1) and maximum (2096) chunks size
const MMAP_THRESHOLD_SHIFT: usize = BLOCK_SHIFT - 3;
const ALLOCATOR_ALIGN: usize = 128;

// derived constants
const MMAP_THRESHOLD: usize = 1 << MMAP_THRESHOLD_SHIFT;

const BLOCK_SIZE: usize = 1 << BLOCK_SHIFT;
const BLOCK_LOWER_MASK: usize = BLOCK_SIZE - 1;

const MAX_CHUNK_SHIFT: usize = BLOCK_SHIFT - 1;
const N_CHUNK_SHIFT_BITS: usize =
    core::mem::size_of::<usize>() * 8 - MAX_CHUNK_SHIFT.leading_zeros() as usize;

const CHUNK_SHIFT_MASK: u8 = (1 << N_CHUNK_SHIFT_BITS) - 1;
const CHUNK_POPULATED_MASK: u8 = 1 << N_CHUNK_SHIFT_BITS;
const CHUNK_FULL_MASK: u8 = 1 << (N_CHUNK_SHIFT_BITS + 1);
const CHUNK_HEADER_MASK: u8 = !(CHUNK_SHIFT_MASK | CHUNK_POPULATED_MASK);

const STATUS_BITS_SIZE: usize = 2;

#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
struct Block([u8; BLOCK_SIZE]);

impl Block {
    #[inline]
    fn chunk_shift(&self) -> u8 {
        self.0[0] & CHUNK_SHIFT_MASK
    }

    #[inline]
    fn chunk_size(&self) -> usize {
        1 << self.chunk_shift() as usize
    }

    #[inline]
    fn n_chunks(&self) -> usize {
        BLOCK_SIZE / self.chunk_size()
    }

    fn free_header_bits(&self) -> usize {
        self.chunk_size() * 8 - N_CHUNK_SHIFT_BITS - STATUS_BITS_SIZE
    }

    #[inline]
    fn n_header_chunks(&self) -> usize {
        // we have 8 => 2^3 bits for each byte so add 3 to the exponent
        let chunk_bit_shift = self.chunk_shift() + 3;

        let free_header_bits = self.free_header_bits();

        let needed_bits = self.n_chunks().saturating_sub(free_header_bits);

        // calculate needed header chunks for all chunks including headers
        let naive_n_headers = ceil_shr(needed_bits, chunk_bit_shift as u32);

        // calculate actual needed headers chunks
        let n_headers = ceil_shr(needed_bits - naive_n_headers, chunk_bit_shift as u32);

        if n_headers < naive_n_headers {
            // we saved one or more chunks

            // make sure the chunks we saved don't require additional header chunks
            let n_headers_with_saved_chunks =
                ceil_shr(needed_bits - n_headers, chunk_bit_shift as u32);

            // if we would require additional chunks, give up and return the naive solution
            if n_headers_with_saved_chunks != n_headers {
                return naive_n_headers + 1;
            }
        }

        // we can fit all free chunks into our optimized headers. (yay)
        n_headers + 1
    }

    #[inline]
    fn first_chunk(&self) -> usize {
        self.n_header_chunks()
    }

    // TODO: optimize these by writing ranges at a time

    #[inline]
    fn get_header_free_bit(&self, j: usize) -> bool {
        let i = j + N_CHUNK_SHIFT_BITS + STATUS_BITS_SIZE;
        let byte = i / 8;
        let offset = i % 8;

        let res = self.0[byte] & (1 << offset);

        // trace!("[{:5}]: {} - {} | {:#010b} => {}", j, byte, offset, self.0[byte], (res != 0) as u8);

        res == 0
    }

    #[inline]
    fn set_header_free_bit(&mut self, j: usize, val: bool) {
        let i = j + N_CHUNK_SHIFT_BITS + STATUS_BITS_SIZE;
        let byte = i / 8;
        let offset = i % 8;

        // trace!("[{:5}]: {} - {} | {} => {:#010b}", j, byte, offset, (!val) as u8, self.0[byte]);

        if val {
            self.0[byte] &= !(1 << offset);
        } else {
            self.0[byte] |= 1 << offset;
        }

        // trace!("[{:5}]: {} - {} |   => {:#010b}", j, byte, offset, self.0[byte]);
    }

    #[inline]
    fn get_chunk_free_bit(&self, j: usize) -> bool {
        let byte = self.chunk_size() + j / 8;
        let offset = j % 8;

        let res = self.0[byte] & (1 << offset);

        /*
        trace!(
            "[{:5}]: {} - {} | {:#010b} => {}",
            j,
            byte,
            offset,
            self.0[byte],
            (res != 0) as u8
        );
        */

        res == 0
    }

    #[inline]
    fn set_chunk_free_bit(&mut self, j: usize, val: bool) {
        let byte = self.chunk_size() + j / 8;
        let offset = j % 8;

        // trace!("[{:5}]: {} - {} | {} => {:#010b}", j, byte, offset, (!val) as u8, self.0[byte]);

        if val {
            self.0[byte] &= !(1 << offset);
        } else {
            self.0[byte] |= 1 << offset;
        }

        if val {
            self.0[byte] &= !(1 << offset);
        } else {
            self.0[byte] |= 1 << offset;
        }

        // trace!("[{:5}]: {} - {} |   => {:#010b}", j, byte, offset, self.0[byte]);
    }

    fn alloc(&mut self, size: usize) -> Option<usize> {
        if self.is_full() {
            return None;
        }

        let n_needed = ceil_shr(size, self.chunk_shift() as u32);

        let n_free_bits = self.n_chunks() - self.n_header_chunks();
        let n_header_bits = self.free_header_bits();

        #[derive(Debug, Clone, Copy)]
        struct FreeRegion {
            start: usize,
            size: usize,
        }

        let mut current = FreeRegion { start: 0, size: 0 };

        let mut best: Option<FreeRegion> = None;

        for i in 0..=n_free_bits {
            // trace!("{} @ {}", current.size, current.start);

            let is_free = if i == n_free_bits {
                false
            } else if i < n_header_bits {
                self.get_header_free_bit(i)
            } else {
                self.get_chunk_free_bit(i - n_header_bits)
            };

            if is_free {
                current.size += 1;
            } else {
                if current.size > 0 {
                    // trace!("[{:5}, {:5}]", current.start, current.start + current.size - 1);

                    if let Some(best) = best.as_mut() {
                        if current.size < best.size {
                            *best = current;
                        }
                    } else {
                        best = Some(current);
                    }

                    if current.size == n_needed {
                        // TODO: uncomment
                        // break;
                    }
                }

                current = FreeRegion {
                    start: i + 1,
                    size: 0,
                };
            }
        }

        if let Some(best) = best {
            for i in best.start..best.start + n_needed {
                if i < n_header_bits {
                    self.set_header_free_bit(i, false);
                } else {
                    self.set_chunk_free_bit(i - n_header_bits, false);
                }
            }

            self.0[0] |= CHUNK_POPULATED_MASK;

            if self.check_if_full() {
                self.0[0] |= CHUNK_FULL_MASK;
            }

            Some((self.first_chunk() + best.start) * self.chunk_size())
        } else {
            None
        }
    }

    fn free(&mut self, offset: usize, size: usize) {
        // dbg!(offset, size);

        let start_chunk = (offset / self.chunk_size()) - self.n_header_chunks();
        let n_chunks = ceil_shr(size, self.chunk_shift() as u32);
        let n_header_bits = self.free_header_bits();

        // dbg!(start_chunk, n_chunks);

        for i in start_chunk..start_chunk + n_chunks {
            if i < n_header_bits {
                self.set_header_free_bit(i, true);
            } else {
                self.set_chunk_free_bit(i - n_header_bits, true);
            }
        }

        self.0[0] &= !CHUNK_FULL_MASK;

        if self.check_if_empty() {
            debug_assert_eq!(self.n_bytes_allocated(), 0);
            self.0[0] &= !CHUNK_POPULATED_MASK;
        } else {
            debug_assert_ne!(self.n_bytes_allocated(), 0);
        }
    }

    fn check_if_empty(&self) -> bool {
        let n_free_bits = self.n_chunks() - self.n_header_chunks();
        let n_embedded = 8 - N_CHUNK_SHIFT_BITS - STATUS_BITS_SIZE;
        let n_remaining_free_bits = n_free_bits - n_embedded;

        if self.0[0] & CHUNK_HEADER_MASK != 0 {
            // debug!("used chunk in embedded bits");
            return false;
        }

        let n_full = n_remaining_free_bits / 8;

        let tail_index = n_full + 1;
        for i in 1..tail_index {
            if self.0[i] != 0 {
                // debug!("used chunk in full bits ({})", i);
                return false;
            }
        }

        let n_in_last = n_remaining_free_bits % 8;
        if self.0[tail_index] & ((1 << n_in_last) - 1) != 0 {
            // debug!("used chunk in tail bits");
            return false;
        }

        // debug!("chunk is empty");

        true
    }

    fn check_if_full(&self) -> bool {
        let n_free_bits = self.n_chunks() - self.n_header_chunks();
        let n_embedded = 8 - N_CHUNK_SHIFT_BITS - STATUS_BITS_SIZE;
        let n_remaining_free_bits = n_free_bits - n_embedded;

        if self.0[0] & CHUNK_HEADER_MASK != CHUNK_HEADER_MASK {
            // debug!("non full chunk in embedded bits");
            return false;
        }

        let n_full = n_remaining_free_bits / 8;

        let tail_index = n_full + 1;
        for i in 1..tail_index {
            if self.0[i] != u8::MAX {
                // debug!("non full chunk in full bits ({})", i);
                return false;
            }
        }

        let n_in_last = n_remaining_free_bits % 8;
        let mask = (1 << n_in_last) - 1;
        if self.0[tail_index] & mask != mask {
            // debug!("non full chunk in tail bits");
            return false;
        }

        // debug!("chunk is full");

        true
    }

    fn n_chunks_used(&self) -> usize {
        let n_free_bits = self.n_chunks() - self.n_header_chunks();
        let n_header_bits = self.free_header_bits();

        let mut n_chunks_in_use = 0;

        for i in 0..n_free_bits {
            let is_free = if i < n_header_bits {
                self.get_header_free_bit(i)
            } else {
                self.get_chunk_free_bit(i - n_header_bits)
            };

            if !is_free {
                n_chunks_in_use += 1;
            }
        }

        n_chunks_in_use
    }

    fn n_bytes_allocated(&self) -> usize {
        self.n_chunks_used() * self.chunk_size()
    }

    fn is_empty(&self) -> bool {
        self.0[0] & CHUNK_POPULATED_MASK == 0
    }

    fn is_full(&self) -> bool {
        self.0[0] & CHUNK_FULL_MASK != 0
    }

    fn align(&self) -> usize {
        self.chunk_size().min(ALLOCATOR_ALIGN)
    }
}

#[inline]
// divides `x` by 2^shift, rounding up
fn ceil_shr(x: usize, shift: u32) -> usize {
    let n = x >> shift;

    // check if any of the lower bits were set => shifted away
    let mask = (1 << shift) - 1;
    if x & mask != 0 {
        n + 1
    } else {
        n
    }
}

/*

const CHUNK_SHIFT: usize = 3;
const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;
const BLOCK_N_CHUNKS: usize = BLOCK_SIZE / CHUNK_SIZE;
const BLOCK_FREE_MASK_SIZE: usize = BLOCK_N_CHUNKS / 8;
const CHUNK_ALIGN: usize = if ALLOCATOR_ALIGN < CHUNK_SIZE {
    ALLOCATOR_ALIGN
} else {
    CHUNK_SIZE
};
 */

/// NOTE: Owns the process `brk`
struct AllocatorInner {
    base: *mut Block,
    brk: *mut Block,

    // freed a block since the last time memory was
    // attempted to be returned to the kernel
    freed: bool,
}

unsafe impl Send for AllocatorInner {}
unsafe impl Sync for AllocatorInner {}

struct Allocator(MaybeUninit<Mutex<AllocatorInner>>);

unsafe impl Send for Allocator {}
unsafe impl Sync for Allocator {}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let res = self.alloc(layout);

        debug_assert_eq!(res.align_offset(layout.align()), 0);

        res
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.dealloc(ptr, layout)
    }
}

/*
const CHUNK_SHIFT: usize = 3;
const CHUNK_SIZE: usize = 1 << CHUNK_SHIFT;
const BLOCK_N_CHUNKS: usize = BLOCK_SIZE / CHUNK_SIZE;
const BLOCK_FREE_MASK_SIZE: usize = BLOCK_N_CHUNKS / 8;
const CHUNK_ALIGN: usize = if ALLOCATOR_ALIGN < CHUNK_SIZE {
    ALLOCATOR_ALIGN
} else {
    CHUNK_SIZE
};
 */

#[global_allocator]
static mut GLOBAL_ALLOCATOR: Allocator = Allocator(MaybeUninit::uninit());

/// Initializes the internal state of the global allocator
/// *must* be called before *any* allocations are made (probably in _start)
/// *must* be called exactly once
pub unsafe fn init() -> SyscallResult<()> {
    let base = syscalls::brk(core::ptr::null())?;

    let align_offset = base.align_offset(ALLOCATOR_ALIGN);

    // Align base to `ALLOCATOR_ALIGN`
    let base = base.add(align_offset);
    syscalls::brk(base)?;

    let base = base as *mut Block;

    GLOBAL_ALLOCATOR = Allocator(MaybeUninit::new(FutexMutex::new(AllocatorInner {
        base,
        brk: base,
        freed: false,
    })));

    Ok(())
}

const fn ordinal_s(x: usize) -> &'static str {
    if x == 1 {
        ""
    } else {
        "s"
    }
}

/// De-initializes the global allocator
/// at the moment this just frees all freeable blocks
/// and prints how much memory was leaked (if any)
pub unsafe fn deinit() -> SyscallResult<()> {
    let mut inner = GLOBAL_ALLOCATOR.lock();

    inner.try_return_mem()?;

    let leaked_blocks = inner.brk.offset_from(inner.base);

    if leaked_blocks != 0 {
        warn!("LEAKED MEMORY:");

        let n_bytes = inner.n_bytes_allocated();

        if n_bytes == 0 {
            panic!(
                "allocator logic is wrong: there are allocated blocks left, but no allocated bytes"
            );
        }

        warn!(
            "program lost {} byte{} during its lifetime, keeping {} block{} ({} bytes) allocated",
            n_bytes,
            ordinal_s(n_bytes),
            leaked_blocks,
            ordinal_s(leaked_blocks as usize),
            leaked_blocks as usize * BLOCK_SIZE,
        );
    }

    Ok(())
}

impl AllocatorInner {
    pub unsafe fn n_bytes_allocated(&self) -> usize {
        let mut n_bytes = 0;

        let n = self.brk.offset_from(self.base) as usize;

        for i in 0..n {
            let block_ptr = self.base.add(i);

            let block = &mut *block_ptr;

            n_bytes += block.n_bytes_allocated();
        }

        n_bytes
    }

    unsafe fn resize_brk(&mut self, n: isize) -> SyscallResult<()> {
        let n_blocks = self.brk.offset_from(self.base);

        trace!(
            "{}\x1b[m brk by {} block{} ({} bytes): {} -> {}",
            if n > 0 {
                "\x1b[32mGrowing"
            } else {
                "\x1b[33mShrinking"
            },
            n.abs(),
            ordinal_s(n.abs() as usize),
            n.abs() as usize * BLOCK_SIZE,
            n_blocks,
            n_blocks + n
        );

        // dbg!(self.n_bytes_allocated());

        self.brk = syscalls::brk(self.brk.offset(n) as *const u8)? as *mut Block;

        Ok(())
    }

    pub unsafe fn try_return_mem(&mut self) -> SyscallResult<()> {
        if self.freed {
            // trace!("attempting to return memory to kernel");

            let last_block = self.brk.sub(1);

            let mut block_ptr = last_block;

            while self.base.offset_from(block_ptr) <= 0 && (&*block_ptr).is_empty() {
                block_ptr = block_ptr.sub(1);
            }

            let brk_offset = block_ptr.offset_from(self.brk) + 1;

            if brk_offset != 0 {
                self.resize_brk(brk_offset)?;
            }

            self.freed = false;
        }

        Ok(())
    }

    unsafe fn alloc_blocks(&mut self, requested: usize, chunk_shift: usize) -> SyscallResult<()> {
        debug_assert!(chunk_shift <= MAX_CHUNK_SHIFT);

        let mut n = requested;

        let mut new_block = Block([0; BLOCK_SIZE]);
        new_block.0[0] = chunk_shift as u8;

        #[allow(clippy::never_loop)]
        let block_ptr = 'outer: loop {
            // try to reuse an existing, but empty block
            let n_blocks = self.brk.offset_from(self.base) as usize;

            for i in 0..n_blocks {
                let block_ptr = self.base.add(i);

                let block = &mut *block_ptr;

                if block.is_empty() {
                    let chunk_n = block_ptr.offset_from(self.base);
                    trace!("reusing existing empty chunk \x1b[34m#{}\x1b[m", chunk_n);

                    *block_ptr = new_block;

                    n -= 1;
                    if n == 0 {
                        break 'outer block_ptr;
                    }
                }
            }

            // allocate a new block
            let block_ptr = self.brk;

            self.resize_brk(n as isize)?;

            for i in 0..n {
                let block_ptr = block_ptr.add(i);
                *block_ptr = new_block;
            }

            break block_ptr;
        };

        {
            let created_block = &*block_ptr;

            let n_chunks = created_block.n_chunks();
            let n_useable = n_chunks - created_block.n_header_chunks();

            let loss = 100. * (1.0 - n_useable as f64 / n_chunks as f64);

            trace!(
                "Allocated {} block{} ({} reused) with {} ({} useable | \x1b[{}m{:.1}%\x1b[m loss) {} byte chunk{}, aligned to {} bits",
                requested,
                ordinal_s(n),
                requested - n,
                n_chunks,
                n_useable,
                if loss < 5. {
                    "32"
                } else if loss < 10. {
                    "33"
                } else {
                    "31"
                },
                loss,
                created_block.chunk_size(),
                ordinal_s(created_block.n_chunks()),
                created_block.align(),
            );
        }

        Ok(())
    }
}

pub fn prefered_chunk_size(layout: &Layout) -> usize {
    let n = layout.size() / layout.align();

    (n * layout.align()).next_power_of_two()
}

impl Allocator {
    unsafe fn lock(&self) -> FutexMutexGuard<'_, AllocatorInner> {
        self.0.assume_init_ref().lock()
    }

    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // trace!("alloc: {:?}", layout);

        let prefered_chunk_size = prefered_chunk_size(&layout);

        let mut inner = self.lock();

        if layout.size() >= MMAP_THRESHOLD {
            let allocation_size = layout.size() + layout.align() + size_of::<*mut u8>();

            let allocation = syscalls::mmap(
                null_mut(),
                allocation_size,
                MProt::READ | MProt::WRITE,
                MMapFlags::ANONYMOUS | MMapFlags::PRIVATE,
                0,
                0,
            );

            let allocation = if let Ok(res) = allocation {
                res
            } else {
                return null_mut();
            };

            let data_ptr = allocation.add(size_of::<*mut u8>());
            let align_offset = data_ptr.align_offset(layout.align());

            // write start of allocation just before the data pointer
            let header_ptr = allocation.add(align_offset) as *mut *mut u8;
            header_ptr.write_unaligned(allocation);

            return data_ptr.add(align_offset);
        }

        let mut allocated = false;
        let res = 'outer: loop {
            let n = inner.brk.offset_from(inner.base) as usize;

            // dbg!(n);

            for i in 0..n {
                let block_ptr = inner.base.add(i);

                let block = &mut *block_ptr;

                if block.chunk_size() == prefered_chunk_size {
                    if let Some(offset) = block.alloc(layout.size()) {
                        let block_ptr = block_ptr as *mut u8;

                        break 'outer block_ptr.add(offset);
                    }
                }
            }

            if allocated {
                for i in 0..n {
                    let block_ptr = inner.base.add(i);

                    let block = &mut *block_ptr;

                    if layout.align() <= block.align() {
                        if let Some(offset) = block.alloc(layout.size()) {
                            let block_ptr = block_ptr as *mut u8;

                            break 'outer block_ptr.add(offset);
                        }
                    }
                }

                unreachable!();
            }

            let chunk_size = prefered_chunk_size;

            let shift = size_of::<usize>() * 8 - chunk_size.leading_zeros() as usize - 1;

            debug_assert_eq!(1 << shift, chunk_size);

            if shift > MAX_CHUNK_SHIFT {
                todo!(
                    "increase MAX_CHUNK_SHIFT? Got a prefered shift of: {}",
                    1 << shift
                );
            }

            let shift = shift.min(MAX_CHUNK_SHIFT);

            let mut n_existing_blocks = 0;

            for i in 0..n {
                let block_ptr = inner.base.add(i);

                let block = &mut *block_ptr;

                if block.chunk_shift() as usize == shift {
                    n_existing_blocks += 1;
                }
            }

            let n_new_blocks = (n_existing_blocks >> 1).max(1);

            if inner.alloc_blocks(n_new_blocks, shift).is_err() {
                break 'outer null_mut();
            }

            allocated = true;
        };

        inner
            .try_return_mem()
            .expect("Failed to return memory to kernel");

        res
    }

    // TODO: do not leak all allocated memory..
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        // trace!("dealloc: {:?} with {:?}", ptr, layout);

        if layout.size() >= MMAP_THRESHOLD {
            // trace!("dealloc: {:?} with {:?}", ptr, layout);

            let allocation_size = layout.size() + layout.align() + size_of::<*mut u8>();

            let allocation_ptr_ptr = ptr.sub(size_of::<*mut u8>()) as *const *mut u8;

            let allocation_ptr = allocation_ptr_ptr.read_unaligned();

            syscalls::munmap(allocation_ptr, allocation_size).expect("Failed to munmap memory");

            return;
        }

        let mut inner = self.lock();

        let offset = ptr.offset_from(inner.base as *mut u8);

        if offset < 0 || ptr.offset_from(inner.brk as *mut u8) >= 0 {
            panic!("tried to free a pointer not inside of the `brk`: {:?}", ptr);
        }

        let offset = offset as usize;

        let block_index = offset / BLOCK_SIZE;
        let offset_in_block = offset % BLOCK_SIZE;

        let block_ptr = inner.base.add(block_index);

        let block = &mut *block_ptr;

        block.free(offset_in_block, layout.size());

        inner.freed = true;
    }
}
