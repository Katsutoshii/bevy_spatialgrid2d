use bevy::prelude::*;
use std::ops::{Index, IndexMut};

use crate::RowCol;

use super::SpatialGridSpec;

/// 2D Grid containing arbitrary data.
#[derive(Clone, Default, Debug, Deref, DerefMut, Resource)]
pub struct SpatialGrid2<T: Sized + Default + Clone> {
    #[deref]
    pub spec: SpatialGridSpec,
    pub cells: Vec<T>,
}
impl<T: Sized + Default + Clone> Index<RowCol> for SpatialGrid2<T> {
    type Output = T;
    fn index(&self, i: RowCol) -> &Self::Output {
        &self.cells[self.flat_index(i)]
    }
}
impl<T: Sized + Default + Clone> IndexMut<RowCol> for SpatialGrid2<T> {
    fn index_mut(&mut self, i: RowCol) -> &mut T {
        let flat_i = self.flat_index(i);
        &mut self.cells[flat_i]
    }
}
impl<T: Sized + Default + Clone + Send + Sync + 'static> SpatialGrid2<T> {
    /// Resize the grid to match the given spec.
    pub fn resize_with(&mut self, spec: SpatialGridSpec) {
        self.spec = spec;
        self.resize();
    }
    /// Resize the grid.
    pub fn resize(&mut self) {
        let num_cells = self.spec.rows as usize * self.spec.cols as usize;
        self.cells.resize(num_cells, T::default());
    }
    /// Resets the grid to all default values.
    pub fn reset(&mut self) {
        self.cells = vec![T::default(); self.cells.len()];
    }

    pub fn get(&self, rowcol: RowCol) -> Option<&T> {
        let index = self.flat_index(rowcol);
        self.cells.get(index)
    }
    pub fn get_mut(&mut self, rowcol: RowCol) -> Option<&mut T> {
        let index = self.flat_index(rowcol);
        self.cells.get_mut(index)
    }
}
