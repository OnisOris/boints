use crate::api::{Algorithm, Color, Seed, Sim, Vec3};

#[derive(Default)]
pub struct BouncingBalls;
impl Algorithm for BouncingBalls {
    fn name(&self) -> &'static str {
        "Bouncing Balls"
    }
    fn setup(&mut self, sim: &mut Sim) {
        let floor = sim.spawn_box(Vec3::new(0.0, -0.4, 0.0), Vec3::new(10.0, 0.4, 10.0));
        sim.set_color(floor, Color::new(0.16, 0.21, 0.30, 1.0));
        for (position, size) in [
            (Vec3::new(-10.0, 1.0, 0.0), Vec3::new(0.25, 1.4, 10.0)),
            (Vec3::new(10.0, 1.0, 0.0), Vec3::new(0.25, 1.4, 10.0)),
            (Vec3::new(0.0, 1.0, -10.0), Vec3::new(10.0, 1.4, 0.25)),
            (Vec3::new(0.0, 1.0, 10.0), Vec3::new(10.0, 1.4, 0.25)),
            (Vec3::new(0.0, 0.6, 0.0), Vec3::new(2.0, 0.6, 2.0)),
        ] {
            let id = sim.spawn_box(position, size);
            sim.set_color(id, Color::new(0.28, 0.34, 0.46, 1.0));
        }
        let mut rng = Seed(42);
        for i in 0..sim.agents {
            // Spaced lattice avoids initially intersecting spheres.
            let p = Vec3::new(
                (i % 8) as f32 * 1.1 - 3.85,
                4.0 + (i / 64) as f32 * 1.2 + rng.range(0.0, 0.2),
                ((i / 8) % 8) as f32 * 1.1 - 3.85,
            );
            let id = sim.spawn_ball(p, 0.4);
            sim.set_velocity(
                id,
                Vec3::new(rng.range(-1.0, 1.0), 0.0, rng.range(-1.0, 1.0)),
            );
            sim.set_color(
                id,
                Color::new(rng.range(0.3, 0.95), rng.range(0.5, 0.9), 0.95, 1.0),
            );
        }
    }
    fn update(&mut self, _sim: &mut Sim, _dt: f32) {}
}
pub fn create() -> Box<dyn Algorithm> {
    Box::new(BouncingBalls)
}
