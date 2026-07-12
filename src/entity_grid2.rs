use std::slice::Iter;

use bevy::{
    app::{App, FixedUpdate, Plugin},
    ecs::{
        component::Component,
        entity::Entity,
        lifecycle::HookContext,
        message::{Message, MessageWriter},
        reflect::ReflectComponent,
        resource::Resource,
        schedule::{InternedSystemSet, IntoScheduleConfigs, ScheduleConfigs, SystemSet},
        system::{Query, Res, ResMut},
        world::DeferredWorld,
    },
    log::{error, warn},
    math::Vec2,
    platform::collections::HashSet,
    prelude::{Deref, DerefMut},
    reflect::Reflect,
    state::{condition::in_state, state::OnEnter},
    utils::default,
};
use bevy_newtonian2d::{PhysicsSimulationState, Position2};

use crate::spec::RowColIterator;
use crate::{Aabb2, RowCol, SpatialGrid2, SpatialGridSpec, SpatialGridState, smallset::SmallSet};

pub struct EntityGridPlugin;
impl Plugin for EntityGridPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EntitySetsGrid::default())
            .add_message::<EntityGridEvent>()
            .configure_sets(FixedUpdate, EntityGridSystem::get_config())
            .add_systems(
                OnEnter(SpatialGridState::Load),
                EntitySetsGrid::resize_on_change,
            )
            .add_systems(
                FixedUpdate,
                GridEntity::update.in_set(EntityGridSystem::UpdateGrid),
            );
    }
}

/// Stores a set of entities in each grid cell.
pub type EntitySet = SmallSet<[Entity; 8]>;

/// System set that allows scheduling systems after the grid updates are complete.
#[derive(SystemSet, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum EntityGridSystem {
    UpdateGrid,
}
impl EntityGridSystem {
    fn get_config() -> ScheduleConfigs<InternedSystemSet> {
        (Self::UpdateGrid).run_if(in_state(PhysicsSimulationState::Running))
    }
}

/// Component to track an entity in the grid.
/// Holds its cell position so it can move/remove itself from the grid.
#[derive(Component, Reflect, Copy, Clone, Default, Debug)]
#[reflect(Component)]
#[component(on_remove = GridEntity::on_remove)]
pub struct GridEntity {
    pub rowcol: Option<RowCol>,
}
impl GridEntity {
    pub fn on_remove(mut world: DeferredWorld, context: HookContext) {
        let grid_entity = *world.get::<GridEntity>(context.entity).unwrap();
        let mut grid = world.resource_mut::<EntitySetsGrid>();
        let remove_event = if let Some(rowcol) = grid_entity.rowcol {
            grid.remove(context.entity, rowcol)
        } else {
            let entity = context.entity;
            warn!("{entity} was missing rowcol on delete");
            None
        };
        if let Some(grid_event) = remove_event {
            world.write_message(grid_event);
        }
    }
    pub fn update(
        mut query: Query<(Entity, &mut Self, &Position2)>,
        mut grid: ResMut<EntitySetsGrid>,
        mut event_writer: MessageWriter<EntityGridEvent>,
    ) {
        for (entity, mut grid_entity, position) in &mut query {
            // If on the grid, update rowcol to match the position.
            if let Some(rowcol) = grid.to_rowcol(position.0) {
                if let Some(event) = grid.update(entity, grid_entity.rowcol, rowcol) {
                    grid_entity.rowcol = event.rowcol;
                    event_writer.write(event);
                }
            }
            // If off the grid, remove the entity from the grid.
            else if let Some(prev_rowcol) = grid_entity.rowcol {
                grid.remove(entity, prev_rowcol);
                grid_entity.rowcol = None;
            }
        }
    }
}

/// Communicates updates to the grid to other systems.
#[derive(Message, Debug)]
pub struct EntityGridEvent {
    pub entity: Entity,
    pub prev_rowcol: Option<RowCol>,
    pub prev_empty: bool,
    pub rowcol: Option<RowCol>,
}
impl Default for EntityGridEvent {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            prev_rowcol: None,
            prev_empty: false,
            rowcol: Some((0, 0)),
        }
    }
}

#[derive(Resource, Default, Deref, DerefMut, Debug)]
pub struct EntitySetsGrid(SpatialGrid2<EntitySet>);

impl EntitySetsGrid {
    pub fn resize_on_change(mut grid: ResMut<Self>, spec: Res<SpatialGridSpec>) {
        grid.resize_with(spec.clone());
    }

    /// Update an entity's position in the grid.
    pub fn update(
        &mut self,
        entity: Entity,
        prev_rowcol: Option<RowCol>,
        rowcol: RowCol,
    ) -> Option<EntityGridEvent> {
        // Remove this entity's old position if it was different.
        let mut prev_empty: bool = false;
        if let Some(prev_rowcol) = prev_rowcol {
            // If in same position, do nothing.
            if prev_rowcol == rowcol {
                return None;
            }

            if let Some(entities) = self.get_mut(prev_rowcol) {
                entities.remove(&entity);
                prev_empty = entities.is_empty();
            }
        }

        if let Some(entities) = self.get_mut(rowcol) {
            entities.insert(entity);
            return Some(EntityGridEvent {
                entity,
                prev_rowcol,
                prev_empty,
                rowcol: Some(rowcol),
            });
        }
        None
    }

    /// Iterate over entities in a radius.
    pub fn iter_entities_in_radius<'a>(
        &'a self,
        position: Vec2,
        radius: f32,
    ) -> EntityRadiusIterator<'a> {
        EntityRadiusIterator::new(&self, RowColIterator::new(self.spec, position, radius))
    }

    /// Remove an entity from the grid entirely.
    pub fn remove(&mut self, entity: Entity, rowcol: RowCol) -> Option<EntityGridEvent> {
        if let Some(entities) = self.get_mut(rowcol) {
            entities.remove(&entity);
            return Some(EntityGridEvent {
                entity,
                prev_rowcol: Some(rowcol),
                prev_empty: entities.is_empty(),
                rowcol: None,
            });
        } else {
            error!("No cell at {:?}.", rowcol)
        }
        None
    }

    /// Get all entities in a given bounding box.
    pub fn get_entities_in_aabb(&self, aabb: &Aabb2) -> Vec<Entity> {
        let mut result: HashSet<Entity> = default();

        for rowcol in self.get_in_aabb(aabb) {
            if let Some(entities) = self.get(rowcol) {
                result.extend(entities.iter());
            }
        }
        result.into_iter().collect()
    }
}

/// Iterates over entities in a given radius.
pub struct EntityRadiusIterator<'a> {
    grid: &'a EntitySetsGrid,

    rowcol: RowCol,
    rowcol_iter: RowColIterator,
    entity_iter: Iter<'a, Entity>,
}
impl<'a> EntityRadiusIterator<'a> {
    pub fn new(grid: &'a EntitySetsGrid, rowcol_iter: RowColIterator) -> Self {
        Self {
            grid,
            rowcol: RowCol::default(),
            rowcol_iter,
            entity_iter: Iter::default(),
        }
    }
}

impl<'a> Iterator for EntityRadiusIterator<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(&entity) = self.entity_iter.next() {
                return Some(entity);
            }

            if let Some(rowcol) = self.rowcol_iter.next() {
                self.rowcol = rowcol;
                self.entity_iter = self.grid[self.rowcol].iter();
                continue;
            }

            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update() {
        let mut grid = SpatialGrid2::<EntitySet> {
            spec: SpatialGridSpec {
                rows: 10,
                cols: 10,
                width: 10.0,
            },
            ..Default::default()
        };
        grid.resize();
        assert_eq!(grid.spec.offset(), Vec2 { x: 50.0, y: 50.0 });
        let rowcol = grid.spec.to_rowcol(Vec2 { x: 0., y: 0. });
        assert_eq!(rowcol, Some((5, 5)));

        assert!(grid.get_mut((5, 5)).is_some());
        assert!(grid.get((5, 5)).is_some());
    }
}
