/// Utilities for making grid shader materials.
use std::marker::PhantomData;

use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::shader::load_shader_library;

use crate::{SpatialGridSpec, SpatialGridState};

/// Plugin for a 2D grid shader.
pub struct SpatialGridShaderMaterialPlugin<M: SpatialGridShaderMaterial>(PhantomData<M>);
impl<M: SpatialGridShaderMaterial> Default for SpatialGridShaderMaterialPlugin<M> {
    fn default() -> Self {
        Self(PhantomData::<M>)
    }
}
impl<M: SpatialGridShaderMaterial> Plugin for SpatialGridShaderMaterialPlugin<M>
where
    MaterialPlugin<M>: Plugin,
{
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "spatialgrid2.wgsl");
        app.add_plugins(MaterialPlugin::<M>::default())
            .init_resource::<SpatialGridShaderAssets<M>>()
            .add_systems(OnEnter(SpatialGridState::Load), M::resize_on_change);
    }
}

/// Trait must be implemented by all Plane shaders.
pub trait SpatialGridShaderMaterial: Material + FromWorld {
    /// Scale factor
    fn scale(spec: &SpatialGridSpec) -> Vec3 {
        spec.scale().extend(1.)
    }

    /// Resize the grid based on the grid spec.
    fn resize(&mut self, spec: &SpatialGridSpec, storage_buffers: &mut Assets<ShaderStorageBuffer>);

    /// When the spec is changed, respawn the visualizer entity with the new size.
    fn resize_on_change(
        spec: Res<SpatialGridSpec>,
        assets: Res<SpatialGridShaderAssets<Self>>,
        mut transform: Single<&mut Transform, With<SpatialGridShaderPlane<Self>>>,
        mut shader_assets: ResMut<Assets<Self>>,
        mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    ) -> Result {
        transform.scale = Self::scale(&spec);
        let material = shader_assets.get_mut(&assets.shader_material).unwrap();
        material.resize(&spec, &mut storage_buffers);
        Ok(())
    }
}

/// Component that marks an entity as a shader plane.
#[derive(Debug, Component, Clone)]
#[component(on_add = SpatialGridShaderPlane::<M>::on_add)]
pub struct SpatialGridShaderPlane<M: SpatialGridShaderMaterial>(PhantomData<M>);
impl<M: SpatialGridShaderMaterial> Default for SpatialGridShaderPlane<M> {
    fn default() -> Self {
        Self(PhantomData::<M>)
    }
}
impl<M: SpatialGridShaderMaterial> SpatialGridShaderPlane<M> {
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let assets = world.resource::<SpatialGridShaderAssets<M>>().clone();
        world.commands().entity(context.entity).insert((
            Mesh3d(assets.mesh),
            MeshMaterial3d(assets.shader_material.clone()),
        ));
    }
}

/// Handles to shader plane assets.
#[derive(Resource, Clone)]
pub struct SpatialGridShaderAssets<M: SpatialGridShaderMaterial> {
    pub mesh: Handle<Mesh>,
    pub shader_material: Handle<M>,
}
impl<M: SpatialGridShaderMaterial> FromWorld for SpatialGridShaderAssets<M> {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh: world.add_asset(Mesh::from(Rectangle {
                half_size: Vec2 { x: 0.5, y: 0.5 },
            })),
            shader_material: {
                let material = M::from_world(world);
                world.add_asset(material)
            },
        }
    }
}
