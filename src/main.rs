#![no_std]
#![no_main]
#![feature(rustc_private)]
#![feature(lang_items)]

use core::alloc::Layout;
use core::{alloc::GlobalAlloc, panic::PanicInfo};
use spin::Mutex;

extern crate alloc;
extern crate libc;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

struct BumpAllocator<const N: usize> {
    stack: [u8; N],
    heap_end: usize,
    allocations: Mutex<usize>,
    next_free: Mutex<usize>,
}

impl<const N: usize> BumpAllocator<N> {
    const fn new() -> Self {
        Self {
            stack: [0; N],
            heap_end: 0,
            allocations: Mutex::new(0),
            next_free: Mutex::new(0),
        }
    }

    unsafe fn init(&mut self) {
        let mut next_free = self.next_free.lock();
        *next_free = self.stack.as_ptr() as usize;
        self.heap_end = *next_free + (N - 1);
    }

    fn print_stats(&self) {
        let next_free = self.next_free.lock();
        let next_free_val = *next_free;
        drop(next_free);

        let s = format!(
            "Bump allocator:\nnext free: {}\nheap end: {}\ndiff: {}\n",
            next_free_val,
            self.heap_end,
            self.heap_end - next_free_val
        );
        unsafe {
            libc::printf(s.as_ptr() as *const _);
        }
    }

    fn try_alloc(&self, amount: usize) -> Option<*mut u8> {
        let mut next_free = self.next_free.lock();

        if *next_free + amount >= self.heap_end {
            return None; // Allocator exhausted
        }

        let free = *next_free;
        *next_free += amount;
        *self.allocations.lock() += 1;

        Some(free as *mut u8)
    }

    fn try_dealloc(&self) {
        let mut allocations = self.allocations.lock();
        if *allocations > 0 {
            *allocations -= 1;
        }

        if *allocations == 0 {
            *self.next_free.lock() = self.stack.as_ptr() as usize;
        }
    }
}

unsafe impl<const N: usize> GlobalAlloc for BumpAllocator<N> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.try_alloc(layout.size() + layout.align()).unwrap()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        self.try_dealloc();
    }
}

#[global_allocator]
static mut ALLOCATOR: BumpAllocator<4096> = BumpAllocator::new();

#[no_mangle]
pub extern "C" fn main(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe { ALLOCATOR.init() };

    let v: Vec<i32> = vec![9, 6, -23];
    let s = String::from("Hello, World!\n");
    let vecinfo = format!("{v:#?}\n");

    unsafe {
        libc::printf(s.as_ptr() as *const _);
        libc::printf(vecinfo.as_ptr() as *const _);
        ALLOCATOR.print_stats();
    }

    0
}

#[panic_handler]
fn my_panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
