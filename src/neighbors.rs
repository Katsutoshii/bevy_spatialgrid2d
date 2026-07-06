use crate::{EntityGridLayer, EntityGridSystem, EntitySetsGrid, GridEntity};
use bevy::ecs::query::QueryData;
use bevy::ecs::schedule::InternedSystemSet;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::math::FloatOrd;
use bevy::prelude::*;
use bevy_newtonian2d::{CircleCollider, PhysicsSimulationState, Position2};
use smallvec::SmallVec;

pub struct NeighborsPlugin;
impl Plugin for NeighborsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(FixedUpdate, NeighborsSystem::get_config())
            .add_systems(FixedUpdate, update.in_set(NeighborsSystem::FindNeighbors));
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy, SystemSet)]
pub enum NeighborsSystem {
    FindNeighbors,
}
impl NeighborsSystem {
    fn get_config() -> ScheduleConfigs<InternedSystemSet> {
        (Self::FindNeighbors)
            .after(EntityGridSystem::UpdateGrid)
            .run_if(in_state(PhysicsSimulationState::Running))
    }
}

#[derive(Component, Clone, Copy, Debug, Reflect, Default)]
pub struct NeighborRadius(pub f32);
impl NeighborRadius {
    /// Returns true iff the given squared distance is within the radius.
    fn in_radius(self, distance_squared: f32) -> bool {
        distance_squared < self.0 * self.0
    }
}

#[derive(Clone, Copy, Debug, Reflect)]
pub struct Neighbor {
    pub entity: Entity,
    pub delta: Vec2,
    pub distance_squared: f32,
}

pub const MAX_NEIGHBORS: usize = 16;
pub const MAX_COLLISIONS: usize = 4;

/// Neighbors for this entity in the current frame.
#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component)]
#[require(NeighborRadius, GridEntity, NeighborLayerMask)]
pub struct Neighbors {
    pub same_layer: SmallVec<[Neighbor; MAX_NEIGHBORS]>,
    pub other_layer: SmallVec<[Neighbor; MAX_NEIGHBORS]>,
}

/// Collisions for this entity in the current frame.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[require(NeighborRadius, CircleCollider, GridEntity, NeighborLayerMask)]
pub struct Collisions {
    pub same_layer: SmallVec<[Neighbor; MAX_NEIGHBORS]>,
    pub other_layer: SmallVec<[Neighbor; MAX_NEIGHBORS]>,
}

/// Collisions for this entity in the current frame.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct NeighborLayerMask(pub SmallVec<[EntityGridLayer; EntityGridLayer::MAX_LAYER.0]>);

#[derive(QueryData)]
pub struct NeighborOtherQueryData {
    position: &'static Position2,
    collider: &'static CircleCollider,
}

#[derive(QueryData)]
pub struct NeighborQueryData {
    entity: Entity,
    grid_entity: &'static GridEntity,
    position: &'static Position2,
    collider: &'static CircleCollider,
    radius: &'static NeighborRadius,
    layer_mask: &'static NeighborLayerMask,
}
impl NeighborQueryDataItem<'_, '_> {
    pub fn update(
        &self,
        query: Query<NeighborOtherQueryData>,
        grid: &EntitySetsGrid,
        neighbors: &mut Neighbors,
        collisions: &mut Collisions,
    ) {
        neighbors.same_layer.clear();
        neighbors.other_layer.clear();
        collisions.same_layer.clear();
        collisions.other_layer.clear();

        let mut layers = self.layer_mask.0.clone();
        layers.push(self.grid_entity.layer);

        for (other_entity, other_layer) in
            grid.iter_entity_layers_in_radius(self.position.0, self.radius.0, &layers)
        {
            if self.entity == other_entity {
                continue;
            }

            if let Ok(other) = query.get(other_entity) {
                let (neighbor_layer, collision_layer) = if self.grid_entity.layer == other_layer {
                    (&mut neighbors.same_layer, &mut collisions.same_layer)
                } else {
                    (&mut neighbors.other_layer, &mut collisions.other_layer)
                };
                let delta = other.position.0 - self.position.0;
                let distance_squared = delta.length_squared();
                if !self.radius.in_radius(distance_squared) {
                    continue;
                }

                let neighbor = Neighbor {
                    entity: other_entity,
                    delta,
                    distance_squared,
                };

                if self
                    .collider
                    .is_colliding(*other.collider, distance_squared)
                    && collision_layer.len() < MAX_COLLISIONS
                {
                    collision_layer.push(neighbor.clone());
                }

                neighbor_layer.push(neighbor);
            }
        }

        neighbors
            .same_layer
            .sort_unstable_by_key(|neighbor| FloatOrd(neighbor.distance_squared));
        neighbors
            .other_layer
            .sort_unstable_by_key(|neighbor| FloatOrd(neighbor.distance_squared));
    }
}

pub fn update(
    mut query: Query<(NeighborQueryData, &mut Neighbors, &mut Collisions)>,
    others_query: Query<NeighborOtherQueryData>,
    grid: Res<EntitySetsGrid>,
) {
    query
        .par_iter_mut()
        .for_each(|(object, mut neighbors, mut collisions)| {
            object.update(others_query, &grid, &mut neighbors, &mut collisions);
        });
}

#[cfg(test)]
mod tests {
    use crate::{Collisions, NeighborRadius, Neighbors, SpatialGrid2dPlugin, SpatialGridSpec};
    use bevy::{
        MinimalPlugins,
        app::App,
        state::app::{AppExtStates, StatesPlugin},
        time::TimeUpdateStrategy,
    };
    use bevy_newtonian2d::{CircleCollider, PhysicsSimulationState, Position2};
    use itertools::Itertools;

    /// cargo test -- neighbors::tests::test_update --nocapture
    #[test]
    fn test_update() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, SpatialGrid2dPlugin))
            .insert_state(PhysicsSimulationState::Running)
            .insert_resource(TimeUpdateStrategy::FixedTimesteps(1))
            .insert_resource(SpatialGridSpec {
                cols: 96,
                rows: 96,
                width: 1.0,
            });
        app.update();

        let step_size = 0.5;
        app.world_mut().spawn_batch(
            (-64..64)
                .cartesian_product(-64..64)
                .filter(|&(x, y)| (x, y) != (0, 0))
                .map(|(x, y)| {
                    (
                        Position2::new(x as f32 * step_size, y as f32 * step_size),
                        Neighbors::default(),
                        NeighborRadius(2.0),
                        Collisions::default(),
                        CircleCollider { radius: 1.0 },
                    )
                }),
        );
        let probe = app
            .world_mut()
            .spawn((
                Position2::new(0.0, 0.0),
                Neighbors::default(),
                NeighborRadius(2.0),
                Collisions::default(),
                CircleCollider { radius: 1.0 },
            ))
            .id();

        app.update();

        assert!(app.world().get::<Neighbors>(probe).is_some());
        assert_eq!(
            app.world()
                .get::<Neighbors>(probe)
                .unwrap()
                .same_layer
                .len(),
            16
        );
    }
}
