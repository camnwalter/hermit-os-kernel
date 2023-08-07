//! Implementation of the HermitCore Allocator for dynamically allocating heap memory
//! in the kernel.

use core::alloc::{GlobalAlloc, Layout};

use align_address::Align;
use hermit_sync::RawInterruptTicketMutex;
use talc::{InitOnOom, Span, Talck};

use crate::HW_DESTRUCTIVE_INTERFERENCE_SIZE;

pub struct LockedAllocator(pub Talck<RawInterruptTicketMutex, InitOnOom>);

impl LockedAllocator {
	#[inline]
	fn align_layout(layout: Layout) -> Layout {
		let size = layout.size().align_up(HW_DESTRUCTIVE_INTERFERENCE_SIZE);
		let align = layout.align().max(HW_DESTRUCTIVE_INTERFERENCE_SIZE);
		Layout::from_size_align(size, align).unwrap()
	}

	pub unsafe fn init(&self, heap_bottom: *mut u8, heap_size: usize) {
		let arena = Span::from_base_size(heap_bottom, heap_size);
		unsafe {
			self.0.talc().init(arena);
		}
	}
}

/// To avoid false sharing, the global memory allocator align
/// all requests to a cache line.
unsafe impl GlobalAlloc for LockedAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let layout = Self::align_layout(layout);
		unsafe { self.0.alloc(layout) }
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		let layout = Self::align_layout(layout);
		unsafe { self.0.dealloc(ptr, layout) }
	}
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
	use core::mem;

	use super::*;

	#[test]
	fn empty() {
		let mut arena: [u8; 0x1000] = [0; 0x1000];
		let allocator: LockedAllocator = LockedAllocator(
			talc::Talc::new(unsafe {
				talc::InitOnOom::new(talc::Span::from_slice(
					arena.as_slice() as *const [u8] as *mut [u8]
				))
			})
			.lock(),
		);

		let layout = Layout::from_size_align(1, 1).unwrap();
		// we have 4 kbyte  memory
		assert!(unsafe { !allocator.alloc(layout.clone()).is_null() });

		let layout = Layout::from_size_align(0x1000, mem::align_of::<usize>()).unwrap();
		let addr = unsafe { allocator.alloc(layout) };
		assert!(addr.is_null());
	}
}
