/*
 * Copyright FalkorDB Ltd. 2023 - present
 * Licensed under the Server Side Public License v1 (SSPLv1).
 */

use std::{mem::MaybeUninit, ptr::null_mut};

use libc::pthread_mutex_t;

use crate::binding::graph::{ConfigOptionField, Config_Option_get};

use super::{
    sparse_matrix::SparseMatrix,
    GraphBLAS::{
        GrB_ALL, GrB_BOOL, GrB_DESC_RSC, GrB_DESC_RSCT0, GrB_DESC_RT0, GrB_DESC_S, GrB_DESC_SCT0,
        GrB_DESC_T0, GrB_Matrix, GrB_Scalar_free, GrB_Scalar_new, GrB_Semiring, GrB_Type,
        GrB_Vector, GrB_Vector_free, GrB_Vector_new, GxB_ANY_PAIR_BOOL, GxB_HYPERSPARSE,
        GxB_LOR_BOOL, GxB_SPARSE,
    },
};

struct CMutex {
    mutex: pthread_mutex_t,
}

impl CMutex {
    fn new() -> Self {
        unsafe {
            let mut mutex = MaybeUninit::uninit();
            libc::pthread_mutex_init(mutex.as_mut_ptr(), null_mut());
            Self {
                mutex: mutex.assume_init(),
            }
        }
    }

    fn lock(&mut self) {
        unsafe {
            libc::pthread_mutex_lock(&mut self.mutex);
        }
    }

    fn unlock(&mut self) {
        unsafe {
            libc::pthread_mutex_unlock(&mut self.mutex);
        }
    }
}

pub struct DeltaMatrix {
    dirty: bool, // TODO: volatile
    matrix: SparseMatrix,
    delta_plus: SparseMatrix,
    delta_minus: SparseMatrix,
    transposed: Option<Box<DeltaMatrix>>,
    mutex: Option<CMutex>,
}

impl DeltaMatrix {
    pub fn new(
        ty: GrB_Type,
        nrows: u64,
        ncols: u64,
        transpose: bool,
    ) -> Self {
        unsafe {
            let mut x = Self {
                dirty: false,
                matrix: SparseMatrix::new(ty, nrows, ncols),
                delta_plus: SparseMatrix::new(GrB_BOOL, nrows, ncols),
                delta_minus: SparseMatrix::new(GrB_BOOL, nrows, ncols),
                transposed: if transpose {
                    let mut x = Box::new(Self {
                        dirty: false,
                        matrix: SparseMatrix::new(ty, ncols, nrows),
                        delta_plus: SparseMatrix::new(GrB_BOOL, ncols, nrows),
                        delta_minus: SparseMatrix::new(GrB_BOOL, ncols, nrows),
                        transposed: None,
                        mutex: None,
                    });
                    x.matrix.set_sparsity(GxB_SPARSE | GxB_HYPERSPARSE);
                    x.delta_plus.set_sparsity(GxB_HYPERSPARSE);
                    x.delta_plus.set_always_hyper();
                    x.delta_minus.set_sparsity(GxB_HYPERSPARSE);
                    x.delta_minus.set_always_hyper();
                    Some(x)
                } else {
                    None
                },
                mutex: Some(CMutex::new()),
            };
            x.matrix.set_sparsity(GxB_SPARSE | GxB_HYPERSPARSE);
            x.delta_plus.set_sparsity(GxB_HYPERSPARSE);
            x.delta_plus.set_always_hyper();
            x.delta_minus.set_sparsity(GxB_HYPERSPARSE);
            x.delta_minus.set_always_hyper();
            x
        }
    }

    pub fn transposed(&mut self) -> Option<&mut Box<DeltaMatrix>> {
        self.transposed.as_mut()
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    fn set_dirty(
        &mut self,
        dirty: bool,
    ) {
        self.dirty = dirty;
        if let Some(t) = self.transposed.as_mut() {
            t.set_dirty(dirty);
        }
    }

    pub fn m(&self) -> &SparseMatrix {
        &self.matrix
    }

    pub fn dp(&self) -> &SparseMatrix {
        &self.delta_plus
    }

    pub fn dm(&self) -> &SparseMatrix {
        &self.delta_minus
    }

    pub fn nrows(&self) -> u64 {
        self.matrix.nrows()
    }

    pub fn ncols(&self) -> u64 {
        self.matrix.ncols()
    }

    pub fn nvals(&self) -> u64 {
        self.matrix.nvals() + self.delta_plus.nvals() - self.delta_minus.nvals()
    }

    pub fn resize(
        &mut self,
        nrows_new: u64,
        ncols_new: u64,
    ) {
        if let Some(t) = self.transposed.as_mut() {
            t.resize(ncols_new, nrows_new);
        }
        self.matrix.resize(nrows_new, ncols_new);
        self.delta_plus.resize(nrows_new, ncols_new);
        self.delta_minus.resize(nrows_new, ncols_new);
    }

    pub fn remove_element_bool(
        &mut self,
        i: u64,
        j: u64,
    ) {
        if let Some(t) = self.transposed.as_mut() {
            t.remove_element_bool(j, i);
        }

        if let Some(true) = self.matrix.extract_element_bool(i, j) {
            self.delta_minus.set_element_bool(i, j, true);
        } else {
            self.delta_plus.remove_element(i, j);
        }
        self.set_dirty(true);
    }

    pub fn set_element_bool(
        &mut self,
        i: u64,
        j: u64,
    ) {
        if let Some(t) = self.transposed.as_mut() {
            t.set_element_bool(j, i);
        }

        if let Some(true) = self.delta_minus.extract_element_bool(i, j) {
            self.delta_minus.remove_element(i, j);
            self.set_dirty(true);
        } else if let None = self.matrix.extract_element_bool(i, j) {
            self.delta_plus.set_element_bool(i, j, true);
            self.set_dirty(true);
        }
    }

    pub fn extract_element_bool(
        &self,
        i: u64,
        j: u64,
    ) -> Option<bool> {
        if let Some(true) = self.delta_plus.extract_element_bool(i, j) {
            Some(true)
        } else if let Some(true) = self.delta_minus.extract_element_bool(i, j) {
            None
        } else {
            self.matrix.extract_element_bool(i, j)
        }
    }

    pub fn remove_elements(
        &mut self,
        mask: GrB_Matrix,
    ) {
        unsafe {
            let mut s = MaybeUninit::uninit();
            GrB_Scalar_new(s.as_mut_ptr(), GrB_BOOL);
            self.delta_plus.assign_scalar(
                mask,
                s.assume_init(),
                GrB_ALL,
                self.nrows(),
                GrB_ALL,
                self.ncols(),
                GrB_DESC_S,
            );
            self.delta_minus.assign(
                mask,
                self.matrix.grb_matrix_ref(),
                GrB_ALL,
                self.nrows(),
                GrB_ALL,
                self.ncols(),
                GrB_DESC_S,
            );
            self.set_dirty(true);
            GrB_Scalar_free(s.as_mut_ptr());
        }
    }

    pub fn clear(&mut self) {
        self.matrix.clear();
        self.delta_plus.clear();
        self.delta_minus.clear();
        self.set_dirty(false);
    }

    pub fn copy(
        &mut self,
        a: &DeltaMatrix,
    ) {
        self.matrix.copy(&a.matrix);
        self.delta_plus.copy(&a.delta_plus);
        self.delta_minus.copy(&a.delta_minus);
    }

    pub fn mxm(
        &mut self,
        semiring: GrB_Semiring,
        m: &DeltaMatrix,
        n: &DeltaMatrix,
    ) {
        unsafe {
            let (mask, desc) = if n.delta_minus.nvals() > 0 {
                let mut mask = SparseMatrix::new(GrB_BOOL, self.nrows(), self.ncols());
                mask.mxm(
                    None,
                    &m.matrix,
                    &n.delta_minus,
                    GxB_ANY_PAIR_BOOL,
                    null_mut(),
                );
                if mask.nvals() > 0 {
                    (Some(mask), GrB_DESC_RSC)
                } else {
                    (None, null_mut())
                }
            } else {
                (None, null_mut())
            };

            let accum = if n.delta_plus.nvals() > 0 {
                let mut accum = SparseMatrix::new(GrB_BOOL, self.nrows(), self.ncols());
                accum.mxm(None, &m.matrix, &n.delta_plus, semiring, null_mut());
                if accum.nvals() > 0 {
                    Some(accum)
                } else {
                    None
                }
            } else {
                None
            };

            self.matrix
                .mxm(mask.as_ref(), &m.matrix, &n.matrix, semiring, desc);
            if let Some(accum) = accum {
                self.matrix
                    .element_wise_add(None, None, Some(&accum), GxB_ANY_PAIR_BOOL);
            }
        }
    }

    pub fn element_wise_add(
        &mut self,
        semiring: GrB_Semiring,
        m: &DeltaMatrix,
        n: &DeltaMatrix,
    ) {
        match (
            m.delta_minus.nvals() > 0 || m.delta_plus.nvals() > 0,
            n.delta_minus.nvals() > 0 || n.delta_plus.nvals() > 0,
        ) {
            (true, true) => {
                self.matrix
                    .element_wise_add(None, Some(&m.export()), Some(&n.export()), semiring);
            }
            (true, false) => {
                self.matrix
                    .element_wise_add(None, Some(&m.export()), Some(&n.matrix), semiring);
            }
            (false, true) => {
                self.matrix
                    .element_wise_add(None, Some(&m.matrix), Some(&n.export()), semiring);
            }
            (false, false) => {
                self.matrix
                    .element_wise_add(None, Some(&m.matrix), Some(&n.matrix), semiring);
            }
        }
    }

    pub fn export(&self) -> SparseMatrix {
        let mut m = SparseMatrix::new(unsafe { GrB_BOOL }, self.nrows(), self.ncols());
        if self.delta_minus.nvals() > 0 {
            m.transpose(
                Some(&self.delta_minus),
                null_mut(),
                Some(&self.matrix),
                unsafe { GrB_DESC_RSCT0 },
            );
        } else {
            m.transpose(None, null_mut(), Some(&self.matrix), unsafe {
                GrB_DESC_RT0
            });
        }
        if self.delta_plus.nvals() > 0 {
            m.element_wise_add(None, None, Some(&self.delta_plus), unsafe {
                GxB_ANY_PAIR_BOOL
            });
        }

        m
    }

    pub fn pending(&self) -> bool {
        if let Some(t) = self.transposed.as_ref() {
            if t.pending() {
                return true;
            }
        }

        self.matrix.pending() || self.delta_plus.pending() || self.delta_minus.pending()
    }

    pub fn wait(
        &mut self,
        force_sync: bool,
    ) {
        if let Some(t) = self.transposed.as_mut() {
            t.wait(force_sync);
        }

        let mut delta_max_pending_changes = MaybeUninit::uninit();
        unsafe {
            Config_Option_get(
                ConfigOptionField::DELTA_MAX_PENDING_CHANGES,
                delta_max_pending_changes.as_mut_ptr(),
            )
        };
        let delta_max_pending_changes = unsafe { delta_max_pending_changes.assume_init() };

        self.sync(force_sync, delta_max_pending_changes);
        self.set_dirty(false);
    }

    fn sync(
        &mut self,
        force_sync: bool,
        delta_max_pending_changes: u64,
    ) {
        if force_sync {
            self.sync_deletions();
            self.sync_additions();
        } else {
            if self.delta_minus.nvals() > delta_max_pending_changes {
                self.sync_deletions();
            }

            if self.delta_plus.nvals() > delta_max_pending_changes {
                self.sync_additions();
            }
        }

        self.matrix.wait();
        self.delta_plus.wait();
        self.delta_minus.wait();
    }

    fn sync_deletions(&mut self) {
        self.matrix
            .transpose(Some(&self.delta_minus), null_mut(), None, unsafe {
                GrB_DESC_RSCT0
            });
        self.delta_minus.clear();
    }

    fn sync_additions(&mut self) {
        unsafe {
            self.matrix.assign(
                self.delta_plus.grb_matrix_ref(),
                self.delta_plus.grb_matrix_ref(),
                GrB_ALL,
                self.nrows(),
                GrB_ALL,
                self.ncols(),
                GrB_DESC_S,
            );
        }
        self.delta_plus.clear();
    }

    pub fn extract_row(
        &self,
        v: GrB_Vector,
        i: u64,
    ) {
        unsafe {
            let mut vmask = MaybeUninit::uninit();
            GrB_Vector_new(vmask.as_mut_ptr(), GrB_BOOL, self.ncols());
            let mask = vmask.assume_init();

            self.delta_minus
                .extract(mask, null_mut(), null_mut(), i, GrB_DESC_T0);
            self.matrix.extract(v, mask, null_mut(), i, GrB_DESC_SCT0);
            self.delta_plus
                .extract(v, mask, GxB_LOR_BOOL, i, GrB_DESC_SCT0);
            GrB_Vector_free(vmask.as_mut_ptr());
        }
    }

    pub fn synchronize(
        &mut self,
        nrows: u64,
        ncols: u64,
    ) {
        if !(self.nrows() < nrows || self.ncols() < ncols) && !self.dirty() {
            return;
        }

        self.mutex.as_mut().unwrap().lock();

        if self.nrows() < nrows || self.ncols() < ncols {
            self.resize(nrows, ncols);
        }

        if self.dirty() {
            self.wait(false);
        }

        self.mutex.as_mut().unwrap().unlock();
    }
}

// to run tests:
// export RUSTFLAGS='-L /opt/homebrew/opt/libomp/lib -lgraphblas -lomp'
// cargo test
#[cfg(test)]
mod tests {
    use crate::graph::matrix::GraphBLAS::{GrB_BOOL, GrB_Mode, GrB_init};

    use super::DeltaMatrix;

    #[test]
    fn test_new_matrix() {
        unsafe { GrB_init(GrB_Mode::GrB_NONBLOCKING) };
        let nrows = 100;
        let ncols = 100;
        let mut a = DeltaMatrix::new(unsafe { GrB_BOOL }, nrows, ncols, false);
        assert_eq!(a.m().nvals(), 0);
        assert_eq!(a.dp().nvals(), 0);
        assert_eq!(a.dm().nvals(), 0);
        assert_eq!(a.nrows(), nrows);
        assert_eq!(a.ncols(), ncols);
        assert_eq!(a.nvals(), 0);
        assert_eq!(a.dirty(), false);
        assert_eq!(a.transposed().is_none(), true);

        let mut a = DeltaMatrix::new(unsafe { GrB_BOOL }, nrows, ncols, true);
        assert_eq!(a.m().nvals(), 0);
        assert_eq!(a.dp().nvals(), 0);
        assert_eq!(a.dm().nvals(), 0);
        assert_eq!(a.nrows(), nrows);
        assert_eq!(a.ncols(), ncols);
        assert_eq!(a.nvals(), 0);
        assert_eq!(a.dirty(), false);
        assert_eq!(a.transposed().is_some(), true);
        assert_eq!(a.transposed().unwrap().m().nvals(), 0);
        assert_eq!(a.transposed().unwrap().dp().nvals(), 0);
        assert_eq!(a.transposed().unwrap().dm().nvals(), 0);
        assert_eq!(a.transposed().unwrap().nrows(), ncols);
        assert_eq!(a.transposed().unwrap().ncols(), nrows);
        assert_eq!(a.transposed().unwrap().nvals(), 0);
        assert_eq!(a.transposed().unwrap().dirty(), false);
        assert_eq!(a.transposed().unwrap().transposed().is_none(), true);
    }
}
