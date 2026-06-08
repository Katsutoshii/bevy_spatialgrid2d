use crate::{EntityGridLayer, EntityGridSystem, EntitySetsGrid, GridEntity};
use bevy::ecs::schedule::InternedSystemSet;
use bevy::platform::collections::HashSet;
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
#[derive(Component, Default, Reflect)]
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

pub fn get_neighbors(
    entity: Entity,
    position: Vec2,
    radius: NeighborRadius,
    other_entities: &HashSet<Entity>,
    others: &Query<(&Position2, &CircleCollider)>,
) -> Vec<Neighbor> {
    let mut neighbors: Vec<Neighbor> = Vec::with_capacity(other_entities.len());

    for &other_entity in other_entities.iter() {
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
    neighbors
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
            let same_layer_entities = grid.get_n_entities_in_radius(
                position.0,
                neighbor_radius.0,
                same_layer,
                MAX_NEIGHBORS,
            );
            for neighbor in get_neighbors(
                entity,
                position.0,
                *neighbor_radius,
                &same_layer_entities,
                &others,
            )
            .into_iter()
            {
                neighbors.same_layer.push(neighbor);
                let (_, other_radius) = others.get(neighbor.entity).unwrap();
                if collider.is_colliding(*other_radius, neighbor.distance_squared)
                    && collisions.same_layer.len() < MAX_COLLISIONS
                {
                    collisions.same_layer.push(neighbor);
                }
            }

            let other_layer_entities = grid.get_n_entities_in_radius(
                position.0,
                neighbor_radius.0,
                &layer_mask.0,
                MAX_NEIGHBORS,
            );
            for neighbor in get_neighbors(
                entity,
                position.0,
                *neighbor_radius,
                &other_layer_entities,
                &others,
            )
            .into_iter()
            {
                neighbors.other_layer.push(neighbor);
                let (_, other_collider) = others.get(neighbor.entity).unwrap();
                if collider.is_colliding(*other_collider, neighbor.distance_squared)
                    && collisions.other_layer.len() < MAX_COLLISIONS
                {
                    collisions.other_layer.push(neighbor);
                }
            }
        },
    )
}
