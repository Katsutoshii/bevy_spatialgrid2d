#![allow(dead_code)]
use std::ops::RangeInclusive;

use bevy::{prelude::*, render::render_resource::ShaderType};

use crate::{Aabb2, RowCol};

/// Specification describing how large the grid is.
#[derive(Resource, Reflect, ShaderType, Copy, Clone, Debug)]
#[reflect(Resource)]
#[repr(C)]
pub struct SpatialGridSpec {
    pub rows: u32,
    pub cols: u32,
    pub width: f32,
}
impl Default for SpatialGridSpec {
    fn default() -> Self {
        Self {
            rows: 64,
            cols: 64,
            width: 32.0,
        }
    }
}
impl SpatialGridSpec {
    /// Discretize a f32 into rowcol space.
    #[inline]
    pub fn discretize(&self, value: f32) -> Option<u32> {
        if value < 0.0 {
            return None;
        }
        Some((value / self.width) as u32)
    }

    /// Discretize a f32 into rowcol space without bounds checking.
    #[inline]
    pub fn discretize_unchecked(&self, value: f32) -> u32 {
        (value / self.width) as u32
    }

    /// Covert row, col to a single index.
    #[inline]
    pub fn flat_index(&self, rowcol: RowCol) -> usize {
        let (row, col) = rowcol;
        row as usize * self.cols as usize + col as usize
    }

    /// Returns (row, col) from a position in world space.
    #[inline]
    pub fn to_rowcol(&self, mut position: Vec2) -> Option<RowCol> {
        position += self.offset();
        let res = (self.discretize(position.y)?, self.discretize(position.x)?);
        if self.in_bounds(res) {
            return Some(res);
        }
        None
    }

    /// Returns (row, col) from a position in world space without bounds checking.
    #[inline]
    pub fn to_rowcol_unchecked(&self, mut position: Vec2) -> RowCol {
        position += self.offset();
        (
            self.discretize_unchecked(position.y),
            self.discretize_unchecked(position.x),
        )
    }

    /// Returns (row, col) from a position in world space.
    pub fn to_rowcol_bilinear(&self, mut position: Vec2) -> Option<RowCol> {
        position += self.offset()
            - Vec2 {
                x: 0.5 * self.width,
                y: 0.5 * self.width,
            };
        let res = (self.discretize(position.y)?, self.discretize(position.x)?);
        if self.in_bounds_bilinear(res) {
            return Some(res);
        }
        None
    }

    /// Returns (row, col) from a position in world space.
    pub fn to_uv(&self, mut position: Vec2) -> Vec2 {
        position += self.offset();
        position / self.width
    }

    /// Returns the world position of the cell coordinate.
    pub fn to_world_position(&self, rowcol: RowCol) -> Vec2 {
        let (row, col) = rowcol;
        Vec2 {
            x: (col as f32 + 0.5) * self.width,
            y: (row as f32 + 0.5) * self.width,
        } - self.offset()
    }

    /// Convert local position [-0.5, 0.5] to world coordinates.
    pub fn uv_to_world_position(&self, position: Vec2) -> Vec2 {
        let position = Vec2::new(position.x, -position.y);
        position * self.scale()
    }

    /// Compute the offset vector for this grid spec.
    pub fn offset(&self) -> Vec2 {
        Vec2 {
            x: self.width * self.cols as f32 / 2.,
            y: self.width * self.rows as f32 / 2.,
        }
    }

    /// Compute the (min, max) position for the grid.
    pub fn world2d_bounds(&self) -> Aabb2 {
        Aabb2 {
            min: -self.offset(),
            max: self.offset(),
        }
    }

    /// Compute the (min, max) position for the grid.
    pub fn world2d_bounds_eps(&self) -> Aabb2 {
        Aabb2 {
            min: -self.offset() + self.width,
            max: self.offset() - self.width,
        }
    }

    pub fn scale(&self) -> Vec2 {
        Vec2 {
            x: self.width * self.cols as f32,
            y: self.width * self.rows as f32,
        }
    }

    /// Returns true if within n from the boundary.
    pub fn is_boundary_n(&self, rowcol: RowCol, n: u32) -> bool {
        let (row, col) = rowcol;
        if row < n || row >= self.rows - n {
            return true;
        }
        if col < n || col >= self.cols - n {
            return true;
        }
        false
    }

    /// Returns true iff the rowcol is on the boundary of the grid.
    pub fn is_near_boundary(&self, rowcol: RowCol) -> bool {
        self.is_boundary_n(rowcol, 2)
    }

    /// Returns true iff the rowcol is on the boundary of the grid.
    pub fn is_boundary(&self, rowcol: RowCol) -> bool {
        self.is_boundary_n(rowcol, 1)
    }

    /// Returns true iff the rowcol is in bounds.
    #[inline]
    pub fn in_bounds(&self, rowcol: RowCol) -> bool {
        let (row, col) = rowcol;
        row < self.rows && col < self.cols
    }

    /// Returns true iff the rowcol is in bounds.
    pub fn in_bounds_bilinear(&self, rowcol: RowCol) -> bool {
        let (row, col) = rowcol;
        row < self.rows - 1 && col < self.cols - 1
    }

    /// Returns the 8 neighboring cells to the given cell rowcol.
    /// Diagonals have distance sqrt(2).
    pub fn neighbors8(&self, rowcol: RowCol) -> [(RowCol, f32); 8] {
        let (row, col) = rowcol;
        [
            ((row + 1, col - 1), 2f32.sqrt()), // Up left
            ((row + 1, col), 1.),              // Up
            ((row + 1, col + 1), 2f32.sqrt()), // Up right
            ((row, col + 1), 1.),              // Right
            ((row - 1, col + 1), 2f32.sqrt()), // Down right
            ((row - 1, col), 1.),              // Down
            ((row - 1, col - 1), 2f32.sqrt()), // Down left
            ((row, col - 1), 1.),              // Left
        ]
    }

    /// Copmutes bilinear neighbor indices and weights for a position.
    pub fn bilinear_neighbors(&self, position: Vec2) -> Option<[(RowCol, f32); 4]> {
        let (row, col) = self.to_rowcol_bilinear(position)?;
        let Vec2 { x, y } = position;
        let Vec2 { x: x1, y: y1 } = self.to_world_position((row, col));
        let Vec2 { x: x2, y: y2 } = self.to_world_position((row + 1, col + 1));
        let w2_recip = 1.0 / (self.width * self.width);
        Some([
            ((row, col), (x2 - x) * (y2 - y) * w2_recip),
            ((row + 1, col), (x2 - x) * (y - y1) * w2_recip),
            ((row, col + 1), (x - x1) * (y2 - y) * w2_recip),
            ((row + 1, col + 1), (x - x1) * (y - y1) * w2_recip),
        ])
    }

    /// Returns the 4 neighboring cells to the given cell rowcol.
    pub fn neighbors4(&self, rowcol: RowCol) -> [RowCol; 4] {
        let (row, col) = rowcol;
        [
            (row + 1, col), // Up
            (row, col + 1), // Right
            (row - 1, col), // Down
            (row, col - 1), // Left
        ]
    }

    /// Get all cells in a given bounding box.
    pub fn get_in_aabb(&self, aabb: &Aabb2) -> Vec<RowCol> {
        let mut results = Vec::default();

        let min_rowcol = self.to_rowcol(aabb.min);
        let max_rowcol = self.to_rowcol(aabb.max);
        if let (Some((min_row, min_col)), Some((max_row, max_col))) = (min_rowcol, max_rowcol) {
            for row in min_row..=max_row {
                for col in min_col..=max_col {
                    if self.in_bounds((row, col)) {
                        results.push((row, col));
                    }
                }
            }
        }
        results
    }

    #[inline]
    pub fn iter_cells_in_radius(&self, center: Vec2, radius: f32) -> RowColIterator {
        RowColIterator::new(*self, center, radius)
    }

    /// Returns a cell's bounding box.
    #[inline]
    pub fn get_cell_aabb2(&self, (r, c): RowCol) -> Aabb2 {
        let min = Vec2::new(c as f32, r as f32) * self.width - self.offset();
        Aabb2 {
            min,
            max: min + self.width,
        }
    }

    /// Returns true if the given cell is withiin the given radius from the center.
    pub fn is_cell_in_radius(&self, center: Vec2, radius: f32, (r, c): RowCol) -> bool {
        let aabb = self.get_cell_aabb2((r, c));
        let closest_point = aabb.clamp2(center);
        center.distance_squared(closest_point) <= radius * radius
    }

    /// Returns a range starting at `center - radius` ending at `center + radius`.
    fn cell_range(&self, center: u32, radius: u32) -> RangeInclusive<u32> {
        let (min, max) = (
            center.saturating_sub(radius),
            (center + radius).min(self.rows),
        );
        min..=max
    }
}

#[derive(Default, Debug, Clone)]
pub struct RowColIterator {
    min: RowCol,
    max: RowCol,
    center: Vec2,
    radius: f32,
    spec: SpatialGridSpec,
    current: RowCol,
}
impl RowColIterator {
    pub fn new(spec: SpatialGridSpec, center: Vec2, radius: f32) -> Self {
        let min = spec.to_rowcol_unchecked(center - radius).max((0, 0));
        let max = spec
            .to_rowcol_unchecked(center + radius)
            .min((spec.rows - 1, spec.cols - 1));
        Self {
            min,
            max,
            center,
            current: min,
            radius,
            spec,
        }
    }

    #[inline]
    fn is_cell_in_radius(&self, rowcol: RowCol) -> bool {
        self.spec
            .is_cell_in_radius(self.center, self.radius, rowcol)
    }
}
impl Iterator for RowColIterator {
    type Item = RowCol;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Try to yield the current rowcol, if it's in the radius.
            if self.current.1 <= self.max.1 {
                let result = self.current;
                self.current.1 += 1;
                if self.is_cell_in_radius(result) {
                    return Some(result);
                }
                continue;
            }

            // Try to get the next row.
            self.current.0 += 1;
            if self.current.0 <= self.max.0 {
                self.current.1 = self.min.1;
                continue;
            }

            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;

    use crate::{RowCol, SpatialGridSpec, spec::RowColIterator};

    #[test]
    fn boundary() {
        let spec = SpatialGridSpec {
            rows: 3,
            cols: 3,
            width: 1.0,
        };
        assert!(spec.is_boundary((0, 0)));
        assert!(spec.is_boundary((0, 1)));
        assert!(spec.is_boundary((0, 2)));
        assert!(spec.is_boundary((2, 2)));
        assert!(!spec.is_boundary((1, 1)));
    }

    #[test]
    fn bilinear() {
        let spec = SpatialGridSpec {
            rows: 3,
            cols: 3,
            width: 1.0,
        };
        // Directly in the middle, use 100% of center (1, 1).
        assert_eq!(
            spec.bilinear_neighbors(Vec2 { x: 0.0, y: 0.0 }),
            Some([((1, 1), 1.0), ((2, 1), 0.0), ((1, 2), 0.0), ((2, 2), 0.0)])
        );
        // In between 4 cells, use event blend of all neighboring cells.
        assert_eq!(
            spec.bilinear_neighbors(Vec2 { x: 0.5, y: 0.5 }),
            Some([
                ((1, 1), 0.25),
                ((2, 1), 0.25),
                ((1, 2), 0.25),
                ((2, 2), 0.25)
            ])
        );
        // Evenly splt along the x axis, blend between values on the same row.
        assert_eq!(
            spec.bilinear_neighbors(Vec2 { x: 0.5, y: 0.0 }),
            Some([((1, 1), 0.5), ((2, 1), 0.0), ((1, 2), 0.5), ((2, 2), 0.0)])
        );
        // Evenly splt along the y axis, blend between values on the same column.
        assert_eq!(
            spec.bilinear_neighbors(Vec2 { x: 0.0, y: 0.5 }),
            Some([((1, 1), 0.5), ((2, 1), 0.5), ((1, 2), 0.0), ((2, 2), 0.0)])
        );
    }

    #[test]
    fn in_radius() {
        let spec = SpatialGridSpec {
            rows: 3,
            cols: 3,
            width: 1.0,
        };
        assert!(spec.is_cell_in_radius(Vec2::new(0.0, 0.0), 2.0, (2, 2)));
    }

    #[test]
    fn iter_rowcol() {
        let spec = SpatialGridSpec {
            rows: 3,
            cols: 3,
            width: 1.0,
        };
        let rcs: Vec<RowCol> = RowColIterator::new(spec, Vec2::new(0.0, 0.0), 1.0).collect();
        assert_eq!(
            rcs,
            vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 1),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2),
            ]
        )
    }
}
