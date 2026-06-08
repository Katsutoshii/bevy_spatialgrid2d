/// Utilities for making grid shader materials.
use std::marker::PhantomData;

use bevy::ecs::lifecycle::HookContext;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::{ecs::world::DeferredWorld, light::NotShadowCaster};

use crate::{SpatialGridSpec, SpatialGridState};

/// Plugin for a 2D grid shader.
pub struct GridShaderMaterialPlugin<M: GridShaderMaterial>(PhantomData<M>);
impl<M: GridShaderMaterial> Default for GridShaderMaterialPlugin<M> {
    fn default() -> Self {
        Self(PhantomData::<M>)
    }
}
impl<M: GridShaderMaterial> Plugin for GridShaderMaterialPlugin<M>
where
    MaterialPlugin<M>: Plugin,
{
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<M>::default())
            .init_resource::<GridShaderAssets<M>>()
            .add_systems(OnEnter(SpatialGridState::Load), M::resize_on_change);
    }
}

/// Trait must be implemented by all Plane shaders.
pub trait GridShaderMaterial: Material + FromWorld {
    /// Scale factor
    fn scale(spec: &SpatialGridSpec) -> Vec3 {
        spec.scale().extend(1.)
    }

    /// Resize the grid based on the grid spec.
    fn resize(&mut self, spec: &SpatialGridSpec, storage_buffers: &mut Assets<ShaderStorageBuffer>);

    /// When the spec is changed, respawn the visualizer entity with the new size.
    fn resize_on_change(
        spec: Res<SpatialGridSpec>,
        assets: Res<GridShaderAssets<Self>>,
        mut transform: Single<&mut Transform, With<GridShaderPlane<Self>>>,
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
#[require(NotShadowCaster)]
#[component(on_add = GridShaderPlane::<M>::on_add)]
pub struct GridShaderPlane<M: GridShaderMaterial>(PhantomData<M>);
impl<M: GridShaderMaterial> Default for GridShaderPlane<M> {
    fn default() -> Self {
        Self(PhantomData::<M>)
    }
}
impl<M: GridShaderMaterial> GridShaderPlane<M> {
    fn on_add(mut world: DeferredWorld, context: HookContext) {
        let assets = world.resource::<GridShaderAssets<M>>().clone();
        world.commands().entity(context.entity).insert((
            Mesh3d(assets.mesh),
            MeshMaterial3d(assets.shader_material.clone()),
        ));
    }
}

/// Handles to shader plane assets.
#[derive(Resource, Clone)]
pub struct GridShaderAssets<M: GridShaderMaterial> {
    pub mesh: Handle<Mesh>,
    pub shader_material: Handle<M>,
}
impl<M: GridShaderMaterial> FromWorld for GridShaderAssets<M> {
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
