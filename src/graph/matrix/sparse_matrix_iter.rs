/*
 * Copyright FalkorDB Ltd. 2023 - present
 * Licensed under the Server Side Public License v1 (SSPLv1).
 */

use std::{mem::MaybeUninit, ptr::null_mut};

use crate::grb_check;

use super::{
    sparse_matrix::SparseMatrix,
    GraphBLAS::{
        GB_Iterator_opaque, GrB_Info, GxB_Iterator_get_BOOL, GxB_rowIterator_attach,
        GxB_rowIterator_getColIndex, GxB_rowIterator_getRowIndex, GxB_rowIterator_nextCol,
        GxB_rowIterator_nextRow, GxB_rowIterator_seekRow,
    },
};

pub struct SparseMatrixIter {
    it: GB_Iterator_opaque,
    depleted: bool,
}
impl SparseMatrixIter {
    pub fn new(
        m: &SparseMatrix,
        min_row: u64,
        max_row: u64,
    ) -> SparseMatrixIter {
        unsafe {
            let mut it = MaybeUninit::uninit();
            grb_check!(GxB_rowIterator_attach(
                it.as_mut_ptr(),
                m.grb_matrix_ref(),
                null_mut()
            ));
            let mut ret = Self {
                it: it.assume_init(),
                depleted: false,
            };
            ret.set_range(min_row, max_row);
            ret
        }
    }

    pub fn set_range(
        &mut self,
        min_row: u64,
        max_row: u64,
    ) {
        unsafe {
            match GxB_rowIterator_seekRow(&mut self.it, min_row) {
                GrB_Info::GxB_EXHAUSTED => self.depleted = true,
                GrB_Info::GrB_NO_VALUE => {
                    let mut info = GrB_Info::GrB_NO_VALUE;
                    while info == GrB_Info::GrB_NO_VALUE
                        && GxB_rowIterator_getRowIndex(&mut self.it) < max_row
                    {
                        info = GxB_rowIterator_nextRow(&mut self.it);
                    }

                    self.depleted = info != GrB_Info::GrB_SUCCESS
                        || GxB_rowIterator_getRowIndex(&mut self.it) > max_row;
                }
                GrB_Info::GrB_SUCCESS => {
                    self.depleted = GxB_rowIterator_getRowIndex(&mut self.it) > max_row;
                }
                _ => {
                    debug_assert!(false, "GxB_rowIterator_seekRow failed");
                }
            }
        }
    }

    pub fn next_bool(
        &mut self,
        max_row: u64,
    ) -> Option<(u64, u64, bool)> {
        unsafe {
            if self.depleted {
                return None;
            }

            let row = GxB_rowIterator_getRowIndex(&mut self.it);
            let col = GxB_rowIterator_getColIndex(&mut self.it);
            let val = GxB_Iterator_get_BOOL(&mut self.it);

            let mut info = GxB_rowIterator_nextCol(&mut self.it);
            if info != GrB_Info::GrB_SUCCESS {
                info = GxB_rowIterator_nextRow(&mut self.it);

                while info == GrB_Info::GrB_NO_VALUE
                    && GxB_rowIterator_getRowIndex(&mut self.it) < max_row
                {
                    info = GxB_rowIterator_nextRow(&mut self.it);
                }

                self.depleted = info != GrB_Info::GrB_SUCCESS
                    || GxB_rowIterator_getRowIndex(&mut self.it) > max_row;
            }

            Some((row, col, val))
        }
    }
}
