//! Owned storage for large, CPU-only scenery caches.

use core::mem::{align_of, size_of};

use esp_hal::{
    peripherals::PSRAM,
    psram::{Psram, PsramConfig},
};

pub struct SceneMemory {
    _ram: Psram,
    next: *mut u8,
    remaining: usize,
}

impl SceneMemory {
    pub fn new(peripheral: PSRAM<'static>) -> Self {
        let ram = Psram::new(peripheral, PsramConfig::default());
        let (next, remaining) = ram.raw_parts();
        Self { _ram: ram, next, remaining }
    }

    pub fn place<T: 'static>(&mut self, init: impl FnOnce() -> T) -> &'static mut T {
        // The HAL owns the mapping for the device's lifetime. We consume each
        // aligned range once, and no DMA buffers or other allocators use it.
        unsafe {
            let target = self.reserve(align_of::<T>(), size_of::<T>()).cast::<T>();
            target.write(init());
            &mut *target
        }
    }

    pub fn place_pixels(&mut self, count: usize) -> &'static mut [u16] {
        let bytes = count.checked_mul(size_of::<u16>()).expect("pixel buffer size");
        // Initialize the reserved region in PSRAM without a stack-sized array.
        unsafe {
            let target = self.reserve(align_of::<u16>(), bytes).cast::<u16>();
            target.write_bytes(0, count);
            core::slice::from_raw_parts_mut(target, count)
        }
    }

    #[cfg(feature = "cooperative-sky")]
    pub fn place_atomic_words(&mut self, count: usize) -> &'static [core::sync::atomic::AtomicU32] {
        use core::sync::atomic::AtomicU32;
        let bytes = count.checked_mul(size_of::<AtomicU32>()).expect("sky buffer size");
        // Each atomic is initialized once, before sharing the region with both cores.
        unsafe {
            let target = self.reserve(align_of::<AtomicU32>(), bytes).cast::<AtomicU32>();
            for index in 0..count {
                target.add(index).write(AtomicU32::new(0));
            }
            core::slice::from_raw_parts(target, count)
        }
    }

    #[cfg(feature = "regional-rail")]
    pub fn copy_from<T: Copy + 'static>(&mut self, initial: &'static T) -> &'static mut T {
        // Copy the immutable template to its final address without a stack temporary.
        unsafe {
            let target = self.reserve(align_of::<T>(), size_of::<T>()).cast::<T>();
            core::ptr::copy_nonoverlapping(initial, target, 1);
            &mut *target
        }
    }

    fn reserve(&mut self, alignment: usize, bytes: usize) -> *mut u8 {
        let padding = self.next.align_offset(alignment);
        let space = padding.checked_add(bytes).expect("scenery alignment");
        assert!(bytes != 0 && !self.next.is_null() && space <= self.remaining,
            "insufficient mapped PSRAM for scenery");
        // Both offsets are checked against the remaining mapped region.
        unsafe {
            let target = self.next.add(padding);
            self.next = self.next.add(space);
            self.remaining -= space;
            target
        }
    }
}
