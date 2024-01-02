/*
 * Copyright FalkorDB Ltd. 2023 - present
 * Licensed under the Server Side Public License v1 (SSPLv1).
 */

use std::alloc::{GlobalAlloc, Layout};
use std::os::raw::c_void;

mod binding;
mod undo_log;

extern "C" {
    static RedisModule_Alloc: Option<extern "C" fn(usize) -> *mut c_void>;
    static RedisModule_Free: Option<extern "C" fn(*mut c_void)>;
}

pub struct FalkorDBAlloc;

unsafe impl GlobalAlloc for FalkorDBAlloc {
    unsafe fn alloc(
        &self,
        layout: Layout,
    ) -> *mut u8 {
        let size = (layout.size() + layout.align() - 1) & (!(layout.align() - 1));
        RedisModule_Alloc.unwrap()(size).cast::<u8>()
    }

    unsafe fn dealloc(
        &self,
        ptr: *mut u8,
        _layout: Layout,
    ) {
        RedisModule_Free.unwrap()(ptr.cast::<c_void>())
    }
}

#[cfg(feature = "falkordb_allocator")]
#[global_allocator]
pub static ALLOC: FalkorDBAlloc = FalkorDBAlloc;
