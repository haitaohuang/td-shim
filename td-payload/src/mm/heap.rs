// Copyright (c) 2022 Intel Corporation
//
// SPDX-License-Identifier: BSD-2-Clause-Patent

use core::panic::PanicInfo;
#[cfg(not(feature = "test_heap_size"))]
use linked_list_allocator::LockedHeap;
#[cfg(feature = "test_heap_size")]
use td_benchmark::Alloc;

#[cfg(feature = "test_heap_size")]
#[global_allocator]
static HEAP: Alloc = Alloc;

#[cfg(not(feature = "test_heap_size"))]
#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[panic_handler]
#[allow(clippy::empty_loop)]
fn panic(_info: &PanicInfo) -> ! {
    use crate::println;

    println!("panic ... {:?}", _info);

    // Report a fatal error to the VMM so it can distinguish a crash from a
    // normal HLT/idle and tear down the TD immediately.
    #[cfg(all(feature = "tdx", not(feature = "no-tdvmcall")))]
    {
        use core::fmt::Write;

        // Format up to 64 bytes of the panic message for the VMM
        let mut buf = [0u8; 64];
        let _ = write!(FmtBuf::new(&mut buf), "{}", _info);
        tdx_tdcall::tdx::tdvmcall_report_fatal_error(0, &buf);
    }

    // Fallback for non-TDX or no-tdvmcall builds
    #[cfg(not(all(feature = "tdx", not(feature = "no-tdvmcall"))))]
    {
        x86_64::instructions::hlt();
        loop {}
    }
}

/// Minimal fmt::Write adapter for a fixed-size byte buffer.
struct FmtBuf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> FmtBuf<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
}

impl<'a> core::fmt::Write for FmtBuf<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buf.len() - self.pos;
        let copy_len = if bytes.len() < remaining {
            bytes.len()
        } else {
            remaining
        };
        self.buf[self.pos..self.pos + copy_len].copy_from_slice(&bytes[..copy_len]);
        self.pos += copy_len;
        Ok(())
    }
}

/// The initialization method for the global heap allocator.
pub fn init_heap(heap_start: u64, heap_size: usize) {
    #[cfg(not(feature = "test_heap_size"))]
    unsafe {
        HEAP.lock().init(heap_start as *mut u8, heap_size);
    }
    #[cfg(feature = "test_heap_size")]
    td_benchmark::HeapProfiling::init(heap_start, heap_size);
}
