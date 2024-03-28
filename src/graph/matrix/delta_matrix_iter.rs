/*
 * Copyright FalkorDB Ltd. 2023 - present
 * Licensed under the Server Side Public License v1 (SSPLv1).
 */

use super::{
    delta_matrix::DeltaMatrix, sparse_matrix_iter::SparseMatrixIter, GraphBLAS::GrB_Index,
};

pub struct DeltaMatrixIter<'a> {
    matrix: Option<&'a DeltaMatrix>,
    m_it: SparseMatrixIter,
    dp_it: SparseMatrixIter,
    min_row: GrB_Index,
    max_row: GrB_Index,
}
impl<'a> DeltaMatrixIter<'a> {
    pub fn attach(
        &mut self,
        m: &'a DeltaMatrix,
    ) {
        self.attach_range(m, 0, u64::MAX);
    }

    pub fn attach_range(
        &mut self,
        m: &'a DeltaMatrix,
        min_row: u64,
        max_row: u64,
    ) {
        self.matrix = Some(m);
        self.min_row = min_row;
        self.max_row = max_row;
        self.m_it = SparseMatrixIter::new(m.m(), min_row, max_row);
        self.dp_it = SparseMatrixIter::new(m.dp(), min_row, max_row);
    }

    pub fn detach(&mut self) {
        self.matrix = None;
    }

    pub fn is_attached(
        &self,
        m: &DeltaMatrix,
    ) -> bool {
        self.matrix.is_some() && std::ptr::eq(self.matrix.unwrap().m(), m.m())
    }

    pub fn iterate_row(
        &mut self,
        row_idx: u64,
    ) {
        self.attach_range(self.matrix.unwrap(), row_idx, row_idx);
    }

    pub fn iterate_range(
        &mut self,
        start_row_idx: u64,
        end_row_idx: u64,
    ) {
        self.attach_range(self.matrix.unwrap(), start_row_idx, end_row_idx);
    }

    pub fn next_bool(&mut self) -> Result<Option<(u64, u64, bool)>, ()> {
        if self.matrix.is_none() {
            return Err(());
        }

        while let Some((i, j, v)) = self.m_it.next_bool(self.max_row) {
            if let None = self.matrix.unwrap().dm().extract_element_bool(i, j) {
                return Ok(Some((i, j, v)));
            }
        }

        while let Some((i, j, v)) = self.dp_it.next_bool(self.max_row) {
            return Ok(Some((i, j, v)));
        }

        Ok(None)
    }

    pub fn reset(&mut self) {
        self.attach_range(self.matrix.unwrap(), self.min_row, self.max_row);
    }
}
