//! The complete API needed by ordinary algorithms. Coordinates are XYZ, Y up.
pub use crate::physics::{ObjectId, ObjectState, Sim};
pub use macroquad::prelude::{Color, Vec3};

pub trait Algorithm {
    fn name(&self) -> &'static str;
    fn setup(&mut self, sim: &mut Sim);
    fn update(&mut self, sim: &mut Sim, dt: f32);
}

/// Small reproducible generator; independent of global graphics/random state.
pub struct Seed(pub u64);
impl Seed {
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        min + (max - min) * ((self.0 >> 40) as f32 / 16777216.0)
    }
}
