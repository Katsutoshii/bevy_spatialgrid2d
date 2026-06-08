use bevy::prelude::*;

/// Represents (row, col) coordinates in the grid.
pub type RowCol = (u32, u32);

/// Extension trait to allow computing distances between RowCols.
pub trait RowColDistance {
    fn distance8(self, other: Self) -> f32;
    fn signed_delta8(self, other: Self) -> Vec2;
}
impl RowColDistance for RowCol {
    /// Distance on a grid with 8-connectivity in cell space.
    fn distance8(self, rowcol2: Self) -> f32 {
        let (row1, col1) = self;
        let (row2, col2) = rowcol2;

        let dx = col2.abs_diff(col1);
        let dy = row2.abs_diff(row1);
        let diagonals = dx.min(dy);
        let straights = dx.max(dy) - diagonals;
        2f32.sqrt() * diagonals as f32 + straights as f32
    }

    /// Signed delta rowcol1 and rowcol2 as a float in cell space.
    fn signed_delta8(self, rowcol2: Self) -> Vec2 {
        let (row1, col1) = self;
        let (row2, col2) = rowcol2;
        Vec2 {
            x: col2 as f32 - col1 as f32,
            y: row2 as f32 - row1 as f32,
        }
    }
}
