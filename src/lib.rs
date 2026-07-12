use bevy::prelude::*;

mod aabb2;
mod entity_grid2;
mod grid2;
mod neighbors;
mod rowcol;
mod smallset;
mod sparse_grid2;
mod spec;

mod grid2_shader;

pub use {
    aabb2::Aabb2,
    grid2::SpatialGrid2,
    rowcol::{RowCol, RowColDistance},
    spec::SpatialGridSpec,
};

#[cfg(feature = "shader")]
pub use grid2_shader::{
    SpatialGridShaderAssets, SpatialGridShaderMaterial, SpatialGridShaderMaterialPlugin,
    SpatialGridShaderPlane,
};

#[cfg(feature = "entity")]
pub use entity_grid2::{EntityGridEvent, EntityGridSystem, EntitySet, EntitySetsGrid, GridEntity};

#[cfg(feature = "neighbors")]
pub use neighbors::{Collisions, Neighbor, NeighborRadius, Neighbors, NeighborsSystem};

#[cfg(feature = "sparse")]
pub use sparse_grid2::SparseSpatialGrid2;

/// Plugin for an spacial entity paritioning grid with optional debug functionality.
pub struct SpatialGrid2dPlugin;
impl Plugin for SpatialGrid2dPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_newtonian2d::PhysicsPlugin>() {
            app.add_plugins(bevy_newtonian2d::PhysicsPlugin);
        }
        app.init_resource::<SpatialGridSpec>()
            .init_state::<SpatialGridState>()
            .add_systems(OnEnter(SpatialGridState::Load), SpatialGridState::on_load)
            .add_systems(
                Update,
                SpatialGridState::on_spec_changed.run_if(resource_changed::<SpatialGridSpec>),
            )
            .add_plugins((entity_grid2::EntityGridPlugin, neighbors::NeighborsPlugin));
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, States)]
pub enum SpatialGridState {
    #[default]
    Load,
    Ready,
}
impl SpatialGridState {
    pub fn on_load(mut next_state: ResMut<NextState<SpatialGridState>>) {
        next_state.set(SpatialGridState::Ready);
    }
    pub fn on_spec_changed(mut next_state: ResMut<NextState<SpatialGridState>>) {
        next_state.set(SpatialGridState::Load);
    }
}
