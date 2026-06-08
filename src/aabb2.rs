use rand::Rng;

use bevy::{
    math::{Vec2, Vec3},
    reflect::Reflect,
};
use std::{mem::swap, ops::Mul};

pub const MAX_SAMPLE: usize = 1 << 10;

/// Axis-aligned bounding box in 2d.
#[derive(Default, PartialEq, Debug, Clone, Reflect)]
pub struct Aabb2 {
    pub min: Vec2,
    pub max: Vec2,
}
impl Aabb2 {
    pub fn enforce_minmax(&mut self) {
        if self.min.x > self.max.x {
            swap(&mut self.min.x, &mut self.max.x);
        }
        if self.min.y > self.max.y {
            swap(&mut self.min.y, &mut self.max.y);
        }
    }
    /// Returns the size of the bounding box.
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }
    /// Returns the center of the bounding box.
    pub fn center(&self) -> Vec2 {
        (self.max + self.min) / 2.
    }
    // Returns true if point is in the bounding box.
    pub fn contains(&self, point: Vec2) -> bool {
        (self.min.x <= point.x && point.x < self.max.x)
            && (self.min.y <= point.y && point.y < self.max.y)
    }
    /// Clamp a 2d vector to the bounding box.
    pub fn clamp2(&self, vec: &mut Vec2) {
        vec.x = vec.x.clamp(self.min.x, self.max.x);
        vec.y = vec.y.clamp(self.min.y, self.max.y);
    }
    /// Clamp a 3d vector (ignoring Z) to the bounding box.
    pub fn clamp3(&self, vec: &mut Vec3) {
        vec.x = vec.x.clamp(self.min.x, self.max.x);
        vec.y = vec.y.clamp(self.min.y, self.max.y);
    }

    /// Generate a random value in the bounding box.
    pub fn sample_uniform(&self) -> Vec2 {
        Vec2::new(
            rand::thread_rng().gen_range(self.min.x..self.max.x),
            rand::thread_rng().gen_range(self.min.y..self.max.y),
        )
    }

    /// Generate a random value in the bounding box.
    pub fn sample_uniform_until<F: Fn(Vec2) -> bool>(&self, f: F) -> Vec2 {
        for _ in 0..MAX_SAMPLE {
            let sample = self.sample_uniform();
            if f(sample) {
                return sample;
            }
        }
        panic!(
            "Unable to select a random position even after {} samples.",
            MAX_SAMPLE
        );
    }
}
impl Mul<f32> for Aabb2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            min: self.min * rhs,
            max: self.max * rhs,
        }
    }
}
