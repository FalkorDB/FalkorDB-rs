/*
 * Copyright FalkorDB Ltd. 2023 - present
 * Licensed under the Server Side Public License v1 (SSPLv1).
 */

use std::{mem::MaybeUninit, ptr::null_mut};

use crate::graph::matrix::GraphBLAS::{
    GrB_Col_extract, GrB_DESC_R, GrB_IDENTITY_BOOL, GrB_Matrix_apply, GrB_Matrix_assign_Scalar,
    GrB_Matrix_clear, GrB_Matrix_eWiseAdd_Semiring, GrB_Matrix_free, GrB_Matrix_removeElement,
    GrB_Matrix_setElement_BOOL, GrB_WaitMode, GrB_mxm, GxB_ALWAYS_HYPER, GxB_Matrix_Option_set,
    GxB_Option_Field,
};

use super::GraphBLAS::{
    GrB_ALL, GrB_BinaryOp, GrB_Descriptor, GrB_Index, GrB_Info, GrB_Matrix, GrB_Matrix_assign,
    GrB_Matrix_extractElement_BOOL, GrB_Matrix_ncols, GrB_Matrix_new, GrB_Matrix_nrows,
    GrB_Matrix_nvals, GrB_Matrix_resize, GrB_Matrix_wait, GrB_Scalar, GrB_Semiring, GrB_Type,
    GrB_Vector, GrB_transpose, GxB_Matrix_Pending,
};

#[macro_export]
macro_rules! grb_check {
    ($exp: expr) => {
        let x = $exp;
        debug_assert_eq!(GrB_Info::GrB_SUCCESS, x);
    };
}

pub struct SparseMatrix(GrB_Matrix);

impl Drop for SparseMatrix {
    fn drop(&mut self) {
        unsafe {
            grb_check!(GrB_Matrix_free(&mut self.0));
        }
    }
}

impl SparseMatrix {
    #[must_use]
    pub fn new(
        ty: GrB_Type,
        nrows: u64,
        ncols: u64,
    ) -> Self {
        unsafe {
            let mut m = MaybeUninit::uninit();
            grb_check!(GrB_Matrix_new(m.as_mut_ptr(), ty, nrows, ncols));
            Self { 0: m.assume_init() }
        }
    }

    pub fn grb_matrix_ref(&self) -> GrB_Matrix {
        self.0
    }

    pub fn grb_matrix(mut self) -> GrB_Matrix {
        let m = self.0;
        self.0 = null_mut();
        m
    }

    pub fn set_always_hyper(&mut self) {
        unsafe {
            grb_check!(GxB_Matrix_Option_set(
                self.0,
                GxB_Option_Field::GxB_HYPER_SWITCH,
                GxB_ALWAYS_HYPER
            ));
        }
    }

    pub fn set_sparsity(
        &mut self,
        sparsity: u32,
    ) {
        unsafe {
            grb_check!(GxB_Matrix_Option_set(
                self.0,
                GxB_Option_Field::GxB_SPARSITY_CONTROL,
                sparsity
            ));
        }
    }

    pub fn nrows(&self) -> u64 {
        unsafe {
            let mut nrows: MaybeUninit<u64> = MaybeUninit::uninit();
            grb_check!(GrB_Matrix_nrows(nrows.as_mut_ptr(), self.0));
            nrows.assume_init()
        }
    }

    pub fn ncols(&self) -> u64 {
        unsafe {
            let mut ncols: MaybeUninit<u64> = MaybeUninit::uninit();
            grb_check!(GrB_Matrix_ncols(ncols.as_mut_ptr(), self.0));
            ncols.assume_init()
        }
    }

    pub fn nvals(&self) -> u64 {
        unsafe {
            let mut nvals: MaybeUninit<u64> = MaybeUninit::uninit();
            grb_check!(GrB_Matrix_nvals(nvals.as_mut_ptr(), self.0));
            nvals.assume_init()
        }
    }

    pub fn resize(
        &mut self,
        nrows_new: u64,
        ncols_new: u64,
    ) {
        unsafe {
            grb_check!(GrB_Matrix_resize(self.0, nrows_new, ncols_new));
        }
    }

    pub fn clear(&mut self) {
        unsafe {
            grb_check!(GrB_Matrix_clear(self.0));
        }
    }

    pub fn copy(
        &mut self,
        matrix: &SparseMatrix,
    ) {
        unsafe {
            if matrix.nvals() > 0 {
                grb_check!(GrB_Matrix_apply(
                    self.0,
                    null_mut(),
                    null_mut(),
                    GrB_IDENTITY_BOOL,
                    matrix.0,
                    GrB_DESC_R
                ));
            } else {
                self.clear();
            }
        }
    }

    pub fn extract_element_bool(
        &self,
        i: u64,
        j: u64,
    ) -> Option<bool> {
        unsafe {
            let mut x = MaybeUninit::uninit();
            let info = GrB_Matrix_extractElement_BOOL(x.as_mut_ptr(), self.0, i, j);
            if info == GrB_Info::GrB_SUCCESS {
                Some(x.assume_init())
            } else if info == GrB_Info::GrB_NO_VALUE {
                None
            } else {
                debug_assert!(
                    false,
                    "GrB_Matrix_extractElement_BOOL failed with error code: {:?}",
                    info
                );
                None
            }
        }
    }

    pub fn set_element_bool(
        &mut self,
        i: u64,
        j: u64,
        x: bool,
    ) {
        unsafe {
            grb_check!(GrB_Matrix_setElement_BOOL(self.0, x, i, j));
        }
    }

    pub fn remove_element(
        &self,
        i: u64,
        j: u64,
    ) {
        unsafe {
            grb_check!(GrB_Matrix_removeElement(self.0, i, j));
        }
    }

    pub fn pending(&self) -> bool {
        unsafe {
            let mut pending: MaybeUninit<bool> = MaybeUninit::uninit();
            grb_check!(GxB_Matrix_Pending(self.0, pending.as_mut_ptr()));
            pending.assume_init()
        }
    }

    pub fn wait(&mut self) {
        unsafe {
            grb_check!(GrB_Matrix_wait(self.0, GrB_WaitMode::GrB_MATERIALIZE));
        }
    }

    pub fn assign(
        &mut self,
        mask: GrB_Matrix,
        n: GrB_Matrix,
        i: *const GrB_Index,
        ni: GrB_Index,
        j: *const GrB_Index,
        nj: GrB_Index,
        desc: GrB_Descriptor,
    ) {
        unsafe {
            grb_check!(GrB_Matrix_assign(
                self.0,
                mask,
                null_mut(),
                n,
                i,
                ni,
                j,
                nj,
                desc,
            ));
        }
    }

    pub fn assign_scalar(
        &mut self,
        mask: GrB_Matrix,
        s: GrB_Scalar,
        i: *const GrB_Index,
        ni: GrB_Index,
        j: *const GrB_Index,
        nj: GrB_Index,
        desc: GrB_Descriptor,
    ) {
        unsafe {
            grb_check!(GrB_Matrix_assign_Scalar(
                self.0,
                mask,
                null_mut(),
                s,
                i,
                ni,
                j,
                nj,
                desc,
            ));
        }
    }

    pub fn mxm(
        &mut self,
        mask: Option<&SparseMatrix>,
        m: &SparseMatrix,
        n: &SparseMatrix,
        semiring: GrB_Semiring,
        desc: GrB_Descriptor,
    ) {
        unsafe {
            grb_check!(GrB_mxm(
                self.0,
                if let Some(mask) = mask {
                    mask.0
                } else {
                    null_mut()
                },
                null_mut(),
                semiring,
                m.0,
                n.0,
                desc,
            ));
        }
    }

    pub fn element_wise_add(
        &mut self,
        mask: Option<&SparseMatrix>,
        m: Option<&SparseMatrix>,
        n: Option<&SparseMatrix>,
        semiring: GrB_Semiring,
    ) {
        unsafe {
            grb_check!(GrB_Matrix_eWiseAdd_Semiring(
                self.0,
                if let Some(mask) = mask {
                    mask.0
                } else {
                    null_mut()
                },
                null_mut(),
                semiring,
                if let Some(m) = m { m.0 } else { self.0 },
                if let Some(n) = n { n.0 } else { self.0 },
                null_mut(),
            ));
        }
    }

    pub fn transpose(
        &mut self,
        mask: Option<&SparseMatrix>,
        accum: GrB_BinaryOp,
        m: Option<&SparseMatrix>,
        desc: GrB_Descriptor,
    ) {
        unsafe {
            grb_check!(GrB_transpose(
                self.0,
                if let Some(m) = mask { m.0 } else { null_mut() },
                accum,
                if let Some(m) = m { m.0 } else { self.0 },
                desc
            ));
        }
    }

    pub fn extract(
        &self,
        v: GrB_Vector,
        mask: GrB_Vector,
        accum: GrB_BinaryOp,
        i: u64,
        desc: GrB_Descriptor,
    ) {
        unsafe {
            grb_check!(GrB_Col_extract(
                v,
                mask,
                accum,
                self.0,
                GrB_ALL,
                self.ncols(),
                i,
                desc
            ));
        }
    }
}
