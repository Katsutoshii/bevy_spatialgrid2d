# `bevy_spatialgrid2d`

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/Katsutoshii/bevy_spatialgrid2d#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_spatialgrid2d.svg)](https://crates.io/crates/bevy_spatialgrid2d)
[![Docs](https://docs.rs/bevy_spatialgrid2d/badge.svg)](https://docs.rs/bevy_spatialgrid2d/latest/bevy_spatialgrid2d/)


Simple Newtonian Physics simulator for Bevy game engine.

This doesn't actually implement any collision calculations, just defines some common types for position, velocity, force, rotation, angular velocity, and torque
and implements propagation to Bevy's `Transform`.

Forces (and Torques) are applied in `PhysicsSystem::ApplyForces` once per frame, so schedule your systems before it.

## Examples

```
cargo run --example boids
```

## Bevy support table

| bevy | bevy_spatialgrid2d |
| ---- | ------------------ |
| 0.18 | 0.18.0             |
