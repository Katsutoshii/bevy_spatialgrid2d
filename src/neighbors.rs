use crate::{EntityGridLayer, EntityGridSystem, EntitySetsGrid, GridEntity};
use bevy::ecs::schedule::InternedSystemSet;
use bevy::prelude::*;
use bevy::{ecs::schedule::ScheduleConfigs, math::FloatOrd};
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

pub fn fill_neighbors(
    entity: Entity,
    position: Vec2,
    radius: NeighborRadius,
    mut other_entities: impl Iterator<Item = Entity>,
    others: &Query<(&Position2, &CircleCollider)>,
    neighbors: &mut SmallVec<[Neighbor; MAX_NEIGHBORS]>,
) {
    while let Some(other_entity) = other_entities.next() {
        if entity == other_entity {
            continue;
        }
        if let Ok((other_position, _)) = others.get(other_entity) {
            let delta = other_position.0 - position;
            let distance_squared = delta.length_squared();
            if radius.in_radius(distance_squared) {
                neighbors.push(Neighbor {
                    entity: other_entity,
                    delta,
                    distance_squared,
                });
            }
        }
    }

    neighbors.sort_by_key(|neighbor| FloatOrd(neighbor.distance_squared));
    neighbors.truncate(MAX_NEIGHBORS);
}

pub fn update(
    mut query: Query<(
        Entity,
        &GridEntity,
        &Position2,
        &CircleCollider,
        &NeighborRadius,
        &NeighborLayerMask,
        &mut Neighbors,
        &mut Collisions,
    )>,
    others: Query<(&Position2, &CircleCollider)>,
    grid: Res<EntitySetsGrid>,
) {
    query.par_iter_mut().for_each(
        |(
            entity,
            grid_entity,
            position,
            collider,
            neighbor_radius,
            layer_mask,
            mut neighbors,
            mut collisions,
        )| {
            neighbors.same_layer.clear();
            neighbors.other_layer.clear();
            collisions.same_layer.clear();
            collisions.other_layer.clear();

            let same_layer = &[grid_entity.layer];
            let same_layer_entities =
                grid.iter_entities_in_radius(position.0, neighbor_radius.0, same_layer);
            fill_neighbors(
                entity,
                position.0,
                *neighbor_radius,
                same_layer_entities,
                &others,
                &mut neighbors.same_layer,
            );
            for neighbor in &neighbors.same_layer {
                let (_, other_radius) = others.get(neighbor.entity).unwrap();
                if collider.is_colliding(*other_radius, neighbor.distance_squared)
                    && collisions.same_layer.len() < MAX_COLLISIONS
                {
                    collisions.same_layer.push(neighbor.clone());
                }
            }

            let other_layer_entities =
                grid.iter_entities_in_radius(position.0, neighbor_radius.0, &layer_mask.0);
            fill_neighbors(
                entity,
                position.0,
                *neighbor_radius,
                other_layer_entities.into_iter(),
                &others,
                &mut neighbors.other_layer,
            );
            for neighbor in &neighbors.other_layer {
                let (_, other_collider) = others.get(neighbor.entity).unwrap();
                if collider.is_colliding(*other_collider, neighbor.distance_squared)
                    && collisions.other_layer.len() < MAX_COLLISIONS
                {
                    collisions.other_layer.push(neighbor.clone());
                }
            }
        },
    );
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
                cols: 128,
                rows: 128,
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
