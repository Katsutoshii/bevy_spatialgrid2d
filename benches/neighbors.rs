//! Benchmark nearest neighbors.
//! `cargo bench --bench neighbors`

use bevy::{
    MinimalPlugins,
    app::App,
    state::app::{AppExtStates, StatesPlugin},
    time::TimeUpdateStrategy,
};
use bevy_newtonian2d::{CircleCollider, PhysicsSimulationState, Position2};
use bevy_spatialgrid2d::{
    Collisions, EntityGridLayer, NeighborLayerMask, NeighborRadius, Neighbors, SpatialGrid2dPlugin,
    SpatialGridSpec,
};

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use itertools::Itertools;

fn neighbor_bench(bencher: &mut Bencher<'_>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin, SpatialGrid2dPlugin))
        .insert_state(PhysicsSimulationState::Running)
        .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
        .insert_resource(SpatialGridSpec {
            cols: 132,
            rows: 132,
            width: 1.0,
        });
    app.update();

    let step_size = 1.0;
    app.world_mut().spawn_batch(
        (-64..64)
            .cartesian_product(-64..64)
            .filter(|&(x, y)| (x, y) != (0, 0))
            .map(|(x, y)| {
                (
                    Position2::new(x as f32 * step_size, y as f32 * step_size),
                    Neighbors::default(),
                    NeighborRadius(4.0),
                    Collisions::default(),
                    CircleCollider { radius: 1.0 },
                    NeighborLayerMask::new(&[EntityGridLayer(0)]),
                )
            }),
    );
    let probe = app
        .world_mut()
        .spawn((
            Position2::new(0.0, 0.0),
            Neighbors::default(),
            NeighborRadius(4.0),
            Collisions::default(),
            CircleCollider { radius: 1.0 },
            NeighborLayerMask::new(&[EntityGridLayer(0)]),
        ))
        .id();

    bencher.iter(|| {
        app.update();

        let neighbors = app.world().get::<Neighbors>(probe).unwrap();
        if neighbors.same_layer.len() != 44 {
            panic!(
                "Invalid neighbor count: {} != {}",
                neighbors.same_layer.len(),
                44
            );
        }
    })
}

fn neighbor_bench_group(c: &mut Criterion) {
    // Group name or individual benchmark id
    c.bench_function("neighbor_bench", neighbor_bench);
}

// Macro to set up the benchmark group and main function
criterion_group!(benches, neighbor_bench_group);
criterion_main!(benches);
