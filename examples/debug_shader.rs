//! Basic example rendering an amoeba.
//! `cargo run --example debug_shader`
use std::f32::consts::PI;

use bevy::color::palettes::css::{DARK_GRAY, WHITE};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::render::{render_resource::AsBindGroup, storage::ShaderBuffer};
use bevy::shader::ShaderRef;
use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    ecs::{lifecycle::HookContext, world::DeferredWorld},
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_newtonian2d::{
    PhysicsMaterial, PhysicsSimulationState, PhysicsSystem, Position2, Velocity2,
};
use bevy_spatialgrid2d::{
    GridEntity, NeighborRadius, SpatialGrid2dPlugin, SpatialGridShaderAssets,
    SpatialGridShaderMaterial, SpatialGridShaderMaterialPlugin, SpatialGridShaderPlane,
    SpatialGridSpec, SpatialGridState,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            FpsOverlayPlugin {
                config: FpsOverlayConfig::default(),
            },
            SpatialGrid2dPlugin,
            GridVisualizerPlugin,
        ))
        .insert_state(PhysicsSimulationState::Running)
        .init_resource::<PlayerAssets>()
        .insert_resource(SpatialGridSpec {
            cols: 32,
            rows: 32,
            width: 16.0,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, (GridVisualizer::update, Player::update))
        .add_systems(
            FixedUpdate,
            Player::fixed_update.before(PhysicsSystem::ApplyForces),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(MainCamera);
    commands.spawn(GridVisualizer);
    commands.spawn(Player);
}

#[derive(Component, Reflect)]
#[require(
    Camera3d::default(),
    Projection::Perspective(PerspectiveProjection {
        fov: PI / 2.0,
        near: 0.1,
        far: 2000.,
        ..default()
    }),
    Transform {
        translation: Vec3::new(0.0, 0.0, 256.0),
        ..default()
    })]
struct MainCamera;

/// A player that can be controlled with WASD and is tracked on the grid.
#[derive(Component, Reflect)]
#[require(
    PhysicsMaterial::default(),
    Name::new("Player"),
    GridEntity,
    NeighborRadius(16.0)
)]
#[component(on_add = Player::on_add)]
struct Player;
impl Player {
    /// Insert required assets for the Boid.
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let PlayerAssets { mesh, material } = world.resource::<PlayerAssets>().clone();
        world
            .commands()
            .entity(context.entity)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }

    /// Draw the neighbor radius.
    fn update(mut gizmos: Gizmos, player: Single<(&Position2, &NeighborRadius), With<Player>>) {
        let (position, radius) = player.into_inner();
        gizmos.circle_2d(
            Isometry2d {
                translation: position.0,
                ..default()
            },
            radius.0,
            WHITE,
        );
    }

    /// Move the player around.
    fn fixed_update(
        time: Res<Time>,
        keyboard_input: Res<ButtonInput<KeyCode>>,
        player: Single<(&Position2, &mut Velocity2), With<Player>>,
        grid_spec: Res<SpatialGridSpec>,
    ) {
        let mut velocity = Velocity2::ZERO;

        if keyboard_input.pressed(KeyCode::KeyA) {
            velocity.0 += Vec2::NEG_X;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            velocity.0 += Vec2::X;
        }
        if keyboard_input.pressed(KeyCode::KeyW) {
            velocity.0 += Vec2::Y;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            velocity.0 += Vec2::NEG_Y;
        }

        let (position, mut next_velocity) = player.into_inner();
        let magnitude = 2.0;
        *next_velocity = Velocity2(velocity.normalize_or_zero()) * magnitude;

        let dt = time.delta_secs() * 60.0;
        let next_position = Position2(position.0 + next_velocity.0 * dt);
        let aabb2 = grid_spec.world2d_bounds();
        if next_position.x < aabb2.min.x || next_position.x > aabb2.max.x {
            next_velocity.x = 0.0;
        }
        if next_position.y < aabb2.min.y || next_position.y > aabb2.max.y {
            next_velocity.y = 0.0;
        }
    }
}

/// Assets required for spawning a visible Boid.
#[derive(Resource, Clone)]
struct PlayerAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}
impl FromWorld for PlayerAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh: world.add_asset(RegularPolygon::new(8.0, 3)),
            material: world.add_asset(StandardMaterial {
                unlit: true,
                ..default()
            }),
        }
    }
}

/// Plugin for visualizing the grid.
/// This plugin reads events from the entity grid and updates the shader's input buffer
/// to light up the cells that have entities.
struct GridVisualizerPlugin;
impl Plugin for GridVisualizerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SpatialGridShaderMaterialPlugin::<GridVisualizerMaterial>::default())
            .add_systems(
                Update,
                GridVisualizerMaterial::update.run_if(in_state(SpatialGridState::Ready)),
            );
    }
}

/// Visualizer for the grid.
#[derive(Component)]
#[require(SpatialGridShaderPlane<GridVisualizerMaterial>, NotShadowCaster)]
#[component(on_add = GridVisualizer::on_add)]
struct GridVisualizer;
impl GridVisualizer {
    /// Initialize the grid with the right scale when resized.
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let scale = world.resource::<SpatialGridSpec>().scale();
        world.commands().entity(context.entity).insert(Transform {
            scale: scale.extend(1.0),
            ..default()
        });
    }

    /// Draw the grid gizmo.
    fn update(mut gizmos: Gizmos, spec: Res<SpatialGridSpec>) {
        gizmos
            .grid_2d(
                Isometry2d::IDENTITY,
                UVec2::new(spec.cols, spec.rows),
                Vec2::splat(spec.width),
                DARK_GRAY,
            )
            .outer_edges();
    }
}

/// Material for rendering the grid with populated cells highlighted.
#[derive(Asset, TypePath, Clone, AsBindGroup)]
struct GridVisualizerMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[uniform(1)]
    size: SpatialGridSpec,
    #[storage(2, read_only)]
    grid_handle: Handle<ShaderBuffer>,
    grid: Vec<u32>,
}
impl FromWorld for GridVisualizerMaterial {
    fn from_world(world: &mut World) -> Self {
        let grid = Vec::default();
        Self {
            color: DARK_GRAY.into(),
            size: SpatialGridSpec::default(),
            grid_handle: world.add_asset(grid.clone()),
            grid,
        }
    }
}
impl SpatialGridShaderMaterial for GridVisualizerMaterial {
    fn resize(&mut self, spec: &SpatialGridSpec, storage_buffers: &mut Assets<ShaderBuffer>) {
        self.size = spec.clone();
        self.grid.resize(spec.rows as usize * spec.cols as usize, 0);
        let mut buffer = storage_buffers.get_mut(&self.grid_handle).unwrap();
        buffer.set_data(self.grid.clone());
    }
}
impl GridVisualizerMaterial {
    /// Update the grid shader material.
    pub fn update(
        grid_spec: Res<SpatialGridSpec>,
        assets: Res<SpatialGridShaderAssets<Self>>,
        mut shader_assets: ResMut<Assets<Self>>,
        mut storage_buffers: ResMut<Assets<ShaderBuffer>>,
        query: Query<(&Position2, &NeighborRadius)>,
    ) {
        let mut material = shader_assets.get_mut(&assets.shader_material).unwrap();
        if material.grid.is_empty() {
            return;
        }
        for (position, radius) in query.iter() {
            for neighbor_rowcol in grid_spec.iter_cells_in_radius(position.0, radius.0) {
                material.grid[grid_spec.flat_index(neighbor_rowcol)] = 1;
            }
            let rowcol = grid_spec.to_rowcol_unchecked(position.0);
            material.grid[grid_spec.flat_index(rowcol)] = 3;
        }
        let mut buffer = storage_buffers.get_mut(&material.grid_handle).unwrap();
        buffer.set_data(material.grid.clone());
    }
}
impl Material for GridVisualizerMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/debug_shader.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
