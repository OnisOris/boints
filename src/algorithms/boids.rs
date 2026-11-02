use crate::api::{Algorithm, Color, ObjectState, Seed, Sim, Vec3};

#[derive(Default)]
pub struct Boids {
    snapshot: Vec<ObjectState>,
}
impl Algorithm for Boids {
    fn name(&self) -> &'static str {
        "Boids 3D"
    }
    fn setup(&mut self, sim: &mut Sim) {
        sim.set_gravity(Vec3::ZERO);
        let mut rng = Seed(7123);
        for i in 0..sim.agents {
            let position = Vec3::new(
                (i % 8) as f32 - 3.5,
                2.0 + ((i / 8) % 8) as f32,
                (i / 64) as f32 - 1.0,
            );
            let id = sim.spawn_ball(position, 0.18);
            sim.set_material(id, 0.1, 0.0);
            sim.set_color(id, Color::new(0.3, rng.range(0.55, 0.95), 0.85, 1.0));
            sim.set_velocity(
                id,
                Vec3::new(
                    rng.range(-2.0, 2.0),
                    rng.range(-2.0, 2.0),
                    rng.range(-2.0, 2.0),
                ),
            );
        }
        self.snapshot.reserve(sim.agents);
    }
    fn update(&mut self, sim: &mut Sim, dt: f32) {
        // Reuse a compact state snapshot; never clone the physics world. O(n²) at 100–200 agents.
        self.snapshot.clear();
        self.snapshot.extend(sim.states());
        for a in &self.snapshot {
            let (mut separation, mut alignment, mut center) = (Vec3::ZERO, Vec3::ZERO, Vec3::ZERO);
            let mut neighbors = 0;
            for b in &self.snapshot {
                let offset = a.position - b.position;
                let distance2 = offset.length_squared();
                if a.id != b.id && distance2 < 9.0 {
                    neighbors += 1;
                    alignment += b.velocity;
                    center += b.position;
                    if distance2 < 1.0 {
                        separation += offset / distance2.max(0.01);
                    }
                }
            }
            let mut acceleration = separation * 3.0;
            if neighbors > 0 {
                let n = neighbors as f32;
                acceleration +=
                    (alignment / n - a.velocity) * 1.2 + (center / n - a.position) * 0.65;
            }
            let offset = a.position - Vec3::new(0.0, 5.0, 0.0);
            // Smooth restoring force on all three axes. Hard limit is only a recovery guard.
            acceleration -= offset * 0.22;
            for axis in 0..3 {
                if offset[axis].abs() > 7.0 {
                    acceleration[axis] -= offset[axis].signum() * (offset[axis].abs() - 7.0) * 4.0;
                }
            }
            let velocity =
                (a.velocity + acceleration.clamp_length_max(18.0) * dt).clamp_length_max(4.0);
            sim.set_velocity(a.id, velocity);
            if offset.abs().max_element() > 12.0 {
                sim.set_position(
                    a.id,
                    Vec3::new(0.0, 5.0, 0.0) + offset.clamp(Vec3::splat(-12.0), Vec3::splat(12.0)),
                );
            }
        }
    }
}
pub fn create() -> Box<dyn Algorithm> {
    Box::<Boids>::default()
}
