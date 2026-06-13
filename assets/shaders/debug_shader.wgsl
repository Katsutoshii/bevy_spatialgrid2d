#import bevy_pbr::{mesh_view_bindings::globals, forward_io::VertexOutput}
#import bevy_spatialgrid2::{SpatialGridSpec, grid_coords, grid_index}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> color: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> spec: SpatialGridSpec;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<storage, read> grid: array<u32>;


@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var output_color = color;
    output_color.a = 0.02;

    let g = grid_coords(spec, mesh.world_position.xy);
    let row = u32(g.y);
    let col = u32(g.x);
    output_color.a += 0.1 * f32(grid[grid_index(spec, row, col)]);

    return output_color;
}
