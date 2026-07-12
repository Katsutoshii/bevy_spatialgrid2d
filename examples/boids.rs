//! Example using SpatialGrid2d to run a Boids simulation.
//! `cargo run --example boids`

use std::f32::consts::PI;

use bevy::{
    DefaultPlugins,
    app::{App, FixedUpdate, Plugin, Startup, Update},
    asset::{DirectAssetAccessExt, Handle},
    camera::{Camera2d, ClearColor},
    color::{Color, palettes::css::DARK_GRAY},
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    ecs::{
        component::Component,
        lifecycle::HookContext,
        name::Name,
        query::{QueryData, Without},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res},
        world::{DeferredWorld, FromWorld, World},
    },
    gizmos::gizmos::Gizmos,
    math::{Isometry2d, Rot2, UVec2, Vec2, primitives::RegularPolygon},
    mesh::{Mesh, Mesh2d},
    reflect::Reflect,
    sprite_render::{ColorMaterial, MeshMaterial2d},
    state::{app::AppExtStates, condition::in_state},
    utils::default,
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_newtonian2d::{
    AngularVelocity2, CircleCollider, Force2, PhysicsMaterial, PhysicsSimulationState,
    PhysicsSystem, Position2, Rotation2, Static, Torque2, Velocity2,
};
use bevy_spatialgrid2d::{
    Collisions, NeighborRadius, Neighbors, SpatialGrid2dPlugin, SpatialGridSpec,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            BoidPlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::default(),
            FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    frame_time_graph_config: FrameTimeGraphConfig {
                        enabled: false,
                        ..default()
                    },
                    ..default()
                },
            },
        ))
        .insert_state(PhysicsSimulationState::Running)
        .insert_resource(SpatialGridSpec {
            cols: 32,
            rows: 32,
            width: 16.0,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<BoidAssets>()
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

/// Spawn a camera and a ton of Boids.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let y_max = 32;
    let x_max = 32;
    let step_size = 12.0;
    for y in -(y_max / 2)..(y_max / 2) {
        for x in -(x_max / 2)..(x_max / 2) {
            commands.spawn((
                Boid::default(),
                Position2::new(x as f32 * step_size, y as f32 * step_size),
                Velocity2::new((x as f32 * 0.01).cos(), (y as f32 * 0.01).sin()),
            ));
        }
    }
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

/// Plugin for an spacial entity paritioning grid with optional debug functionality.
pub struct BoidPlugin;
impl Plugin for BoidPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SpatialGrid2dPlugin).add_systems(
            FixedUpdate,
            Boid::fixed_update
                .before(PhysicsSystem::ApplyForces)
                .run_if(in_state(PhysicsSimulationState::Running)),
        );
    }
}

/// Assets required for spawning a visible Boid.
#[derive(Resource, Clone)]
struct BoidAssets {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}
impl FromWorld for BoidAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh: world.add_asset(RegularPolygon::new(4.0, 3)),
            material: world.add_asset(ColorMaterial::from_color(Color::WHITE)),
        }
    }
}

/// Boid struct containing parameters for the boid simulation.
#[derive(Component, Reflect)]
#[require(
    Neighbors,
    NeighborRadius(64.0),
    Collisions,
    CircleCollider { radius: 4.0 },
    PhysicsMaterial {
        friction: 0.01,
        ..default()
    },
    Name::new("Boid")
)]
#[component(on_add = Boid::on_add)]
pub struct Boid {
    pub separation_radius_factor: f32,
    pub separation_force_factor: f32,
    pub alignment_force_factor: f32,
    pub cohesion_force_factor: f32,
    pub boundary_force_factor: f32,
    pub global_force_factor: f32,
    pub vortex_force_factor: f32,
    pub torque_factor: f32,
    pub angular_velocity_break: f32,
}
impl Default for Boid {
    fn default() -> Self {
        Self {
            separation_radius_factor: 5.0,
            separation_force_factor: 0.8,
            alignment_force_factor: 0.3,
            cohesion_force_factor: 0.05,
            boundary_force_factor: 0.4,
            global_force_factor: 0.1,
            vortex_force_factor: 0.001,
            torque_factor: 0.03,
            angular_velocity_break: 30.0,
        }
    }
}
impl Boid {
    /// Insert required assets for the Boid.
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let BoidAssets { mesh, material } = world.resource::<BoidAssets>().clone();
        world
            .commands()
            .entity(context.entity)
            .insert((Mesh2d(mesh), MeshMaterial2d(material)));
    }

    /// Fixed update Boid simulation.
    fn fixed_update(
        mut query: Query<(BoidQueryData, &mut Force2, &mut Torque2), Without<Static>>,
        others: Query<(&Boid, &CircleCollider, &Velocity2)>,
        grid_spec: Res<SpatialGridSpec>,
    ) {
        query
            .par_iter_mut()
            .for_each(|(boid_query_data, mut force, mut torque)| {
                *force += boid_query_data.compute_forces(&others, &grid_spec);
                *torque += boid_query_data.compute_torque();
            });
    }

    /// Compute force from separation.
    /// The direction is towards self away from each nearby bird.
    /// The magnitude is computed by
    /// $magnitude = sep * (-x^2 / r^2 + 1)$
    fn separation_force(
        &self,
        collider: CircleCollider,
        delta_norm: Vec2,
        distance_squared: f32,
    ) -> Force2 {
        let separation_radius = self.separation_radius_factor * collider.radius;
        let separation_radius_squared = separation_radius * separation_radius;
        let magnitude = self.separation_force_factor
            * (separation_radius_squared - distance_squared)
            / separation_radius_squared;
        Force2(delta_norm * magnitude.clamp(0.0, 10.0))
    }

    /// Compute force for alignment.
    /// This is based on the difference between this object's velocity and the other object's velocity.
    fn alignment_force(
        &self,
        distance_squared: f32,
        radius_squared: f32,
        velocity: Velocity2,
        other_velocity: Velocity2,
    ) -> Force2 {
        let magnitude = (radius_squared - distance_squared) / radius_squared;
        Force2((other_velocity.0 - velocity.0) * self.alignment_force_factor * magnitude)
    }

    /// Compute force for cohesion.
    fn cohesion_force(&self, delta_norm: Vec2) -> Force2 {
        Force2(self.cohesion_force_factor * delta_norm)
    }

    /// Computes the force from being near the boundary.
    fn boundary_force(&self, position: &Position2, grid_spec: &SpatialGridSpec) -> Force2 {
        let aabb2 = grid_spec.world2d_bounds();
        let w = grid_spec.width;
        let mut force = Force2::ZERO;
        if position.x < aabb2.min.x + w {
            force.x += aabb2.min.x + w - position.x;
        } else if position.x > aabb2.max.x - w {
            force.x += aabb2.max.x - w - position.x;
        }
        if position.y < aabb2.min.y + w {
            force.y += aabb2.min.y + w - position.y;
        } else if position.y > aabb2.max.y - w {
            force.y += aabb2.max.y - w - position.y;
        }
        force * self.boundary_force_factor
    }

    /// Compute vortex force.
    /// This spins the Boids around the origin.
    fn vortex_force(&self, position: &Position2) -> Force2 {
        let spin_force = Force2(Vec2::from_angle(PI / 2.0).rotate(position.0));
        let pull_force = Force2(-position.0);
        (spin_force + pull_force) * self.vortex_force_factor
    }
}

/// All components required for computing Boid forces.
#[derive(QueryData)]
struct BoidQueryData {
    boid: &'static Boid,
    position: &'static Position2,
    velocity: &'static Velocity2,
    rotation: &'static Rotation2,
    angular_velocity: &'static AngularVelocity2,
    collider: &'static CircleCollider,
    neighbor_radius: &'static NeighborRadius,
    neighbors: &'static Neighbors,
}
impl BoidQueryDataItem<'_, '_> {
    /// Computes Boid forces.
    fn compute_forces(
        &self,
        others: &Query<(&Boid, &CircleCollider, &Velocity2)>,
        grid_spec: &SpatialGridSpec,
    ) -> Force2 {
        let mut separation_force = Force2::ZERO;
        let mut alignment_force = Force2::ZERO;
        let mut cohesion_force = Force2::ZERO;

        // Boids in the same layer apply separation, alignment, and cohesion forces.
        for neighbor in self.neighbors.iter() {
            let Ok((other_boid, other_radius, other_velocity)) = others.get(neighbor.entity) else {
                continue;
            };
            let neighbor_radius_squared = self.neighbor_radius.0 * self.neighbor_radius.0;
            let delta_norm = neighbor.delta.normalize_or_zero();

            separation_force +=
                other_boid.separation_force(*other_radius, -delta_norm, neighbor.distance_squared);
            alignment_force += other_boid.alignment_force(
                neighbor.distance_squared,
                neighbor_radius_squared,
                *self.velocity,
                *other_velocity,
            );
            cohesion_force += other_boid.cohesion_force(delta_norm);
        }

        let mut total_force = Force2::ZERO;
        if !self.neighbors.is_empty() {
            let neighbor_count = self.neighbors.len() as f32 + 1.0;
            total_force += alignment_force * (1.0 / neighbor_count);
            total_force += cohesion_force * (1.0 / neighbor_count);
        }
        total_force += separation_force;
        total_force += self.boid.boundary_force(self.position, grid_spec);
        total_force += self.boid.vortex_force(self.position);
        total_force * self.boid.global_force_factor
    }

    /// Computes torque on the Boid.
    fn compute_torque(&self) -> Torque2 {
        (Torque2::towards(
            *self.rotation,
            Rotation2(Rot2 {
                cos: self.velocity.0.x,
                sin: self.velocity.0.y,
            }),
        ) - (*self.angular_velocity * self.boid.angular_velocity_break))
            * self.boid.torque_factor
    }
}
