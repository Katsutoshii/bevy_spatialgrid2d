#define_import_path bevy_spatialgrid2

// Specifies grid size.
struct SpatialGridSpec {
    rows: u32,
    cols: u32,
    width: f32,
};

/// Computes the grid index from row, col.
fn grid_index(spec: SpatialGridSpec, row: u32, col: u32) -> u32 {
    return row * spec.cols + col;
}

/// Computes the offset from the given coordinates.
fn grid_offset(spec: SpatialGridSpec) -> vec2<f32> {
    return vec2<f32>(
        f32(spec.cols),
        f32(spec.rows)
    ) * spec.width / 2.;
}

/// Get fractional rowcol coords from world position.
fn grid_coords(spec: SpatialGridSpec, position: vec2<f32>) -> vec2<f32> {
    return (position + grid_offset(spec)) / spec.width;
}

/// Get fractional rowcol coords from UV coordinates.
fn grid_uv(spec: SpatialGridSpec, uv: vec2<f32>) -> vec2<f32> {
    var flipped_uv = uv;
    flipped_uv.y = 1. - uv.y;
    return flipped_uv * vec2<f32>(
        f32(spec.cols),
        f32(spec.rows)
    );
}