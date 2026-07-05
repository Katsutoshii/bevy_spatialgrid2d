//! Benchmark nearest neighbors.
//! `cargo bench --bench neighbors`
use std::hint::black_box;

use bevy::{
    MinimalPlugins,
    app::App,
    state::app::{AppExtStates, StatesPlugin},
    time::{Time, TimeUpdateStrategy},
};
use bevy_newtonian2d::{CircleCollider, PhysicsSimulationState, Position2};
use bevy_spatialgrid2d::{
    Collisions, NeighborRadius, Neighbors, SpatialGrid2dPlugin, SpatialGridSpec,
};

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use itertools::Itertools;

fn neighbor_bench(bencher: &mut Bencher<'_>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, SpatialGrid2dPlugin))
        .insert_state(PhysicsSimulationState::Running)
        .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
        .insert_resource(SpatialGridSpec {
            cols: 128,
            rows: 128,
            width: 4.0,
        });
    let step_size = 1.0;
    app.world_mut()
        .spawn_batch((0..128).cartesian_product(0..128).map(|(x, y)| {
            (
                Position2::new(x as f32 * step_size, y as f32 * step_size),
                Neighbors::default(),
                NeighborRadius(2.0),
                Collisions::default(),
                CircleCollider { radius: 1.0 },
            )
        }));

    bencher.iter(|| {
        app.update();
        black_box(app.world().resource::<Time>().elapsed());
    })
}

fn neighbor_bench_group(c: &mut Criterion) {
    // Group name or individual benchmark id
    c.bench_function("neighbor_bench", neighbor_bench);
}

// Macro to set up the benchmark group and main function
criterion_group!(benches, neighbor_bench_group);
criterion_main!(benches);
