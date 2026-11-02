use algorithm_playground::{
    algorithms::ALGORITHMS,
    api::{Algorithm, Sim, Vec3},
    physics::DT,
    simulation::Simulation,
};

#[test]
fn gravity_and_floor_collision() {
    let mut sim = Sim::new(1);
    let ball = sim.spawn_ball(Vec3::new(0.0, 5.0, 0.0), 0.5);
    sim.spawn_box(Vec3::new(0.0, -0.5, 0.0), Vec3::new(10.0, 0.5, 10.0));
    for _ in 0..30 {
        sim.step();
    }
    assert!(sim.state(ball).unwrap().position.y < 4.0);
    for _ in 0..600 {
        sim.step();
        assert!(sim.state(ball).unwrap().position.y > 0.35);
    }
}

struct Custom;
impl Algorithm for Custom {
    fn name(&self) -> &'static str {
        "test"
    }
    fn setup(&mut self, sim: &mut Sim) {
        sim.set_gravity(Vec3::ZERO);
        sim.spawn_ball(Vec3::ZERO, 0.5);
    }
    fn update(&mut self, sim: &mut Sim, _: f32) {
        let id = sim.ids().next().unwrap();
        sim.set_velocity(id, Vec3::new(2.0, 1.0, -1.0));
    }
}
#[test]
fn custom_algorithm_creates_and_moves_body() {
    let mut sim = Sim::new(1);
    let mut algorithm = Custom;
    algorithm.setup(&mut sim);
    algorithm.update(&mut sim, DT);
    sim.step();
    let state = sim.states().next().unwrap();
    assert!((state.position.x - 2.0 * DT).abs() < 0.001);
    assert!(state.position.y > 0.0 && state.position.z < 0.0);
}
#[test]
fn reset_recreates_scene_and_invalidates_ids() {
    let mut session = Simulation::new(32);
    let initial: Vec<_> = session.sim.states().map(|s| s.position).collect();
    let old = session.sim.ids().next().unwrap();
    session.step();
    session.reset();
    assert_eq!(session.sim.time, 0.0);
    assert!(session.sim.state(old).is_none());
    assert_eq!(
        initial,
        session.sim.states().map(|s| s.position).collect::<Vec<_>>()
    );
}
#[test]
fn pause_does_not_accumulate_and_step_is_exact() {
    let mut session = Simulation::new(10);
    session.frame(DT * 0.5);
    session.set_playing(false);
    for _ in 0..100 {
        session.frame(1.0);
    }
    assert_eq!(session.sim.time, 0.0);
    session.single_step();
    assert_eq!(session.sim.time, DT as f64);
    session.frame(10.0);
    assert_eq!(session.sim.time, DT as f64);
    session.set_playing(true);
    session.frame(DT * 0.75);
    assert_eq!(session.sim.time, DT as f64);
}
#[test]
fn accumulator_is_fps_independent_and_bounded() {
    for fps in [30, 60, 120] {
        let mut session = Simulation::new(1);
        for _ in 0..fps {
            session.frame(1.0 / fps as f32);
        }
        assert!((session.sim.time - 1.0).abs() < 0.001);
    }
    let mut session = Simulation::new(1);
    session.frame(60.0);
    assert_eq!(session.sim.time, 0.0);
    session.speed = 2.0;
    session.frame(0.25);
    assert!(session.sim.time <= 8.0 * DT as f64 + 0.001);
}
#[test]
fn removed_handles_cannot_alias_replacements() {
    let mut sim = Sim::new(1);
    let old = sim.spawn_ball(Vec3::ZERO, 0.5);
    assert!(sim.remove(old));
    let new = sim.spawn_ball(Vec3::Y, 0.5);
    assert_ne!(old, new);
    assert!(sim.state(old).is_none());
    assert!(!sim.set_velocity(old, Vec3::ONE));
    assert_eq!(sim.len(), 1);
}
#[test]
fn force_is_applied_for_one_step_and_invalid_values_rejected() {
    let mut sim = Sim::new(1);
    sim.set_gravity(Vec3::ZERO);
    let id = sim.spawn_ball(Vec3::ZERO, 0.5);
    assert!(sim.apply_force(id, Vec3::X * 10.0));
    sim.step();
    let velocity = sim.state(id).unwrap().velocity.x;
    assert!(velocity > 0.0);
    sim.step();
    assert!((sim.state(id).unwrap().velocity.x - velocity).abs() < 0.0001);
    assert!(!sim.set_velocity(id, Vec3::splat(f32::NAN)));
    assert!(!sim.set_position(id, Vec3::splat(f32::INFINITY)));
}
#[test]
fn boids_are_finite_bounded_and_move_in_three_dimensions() {
    let mut session = Simulation::new(100);
    let index = ALGORITHMS
        .iter()
        .position(|create| create().name() == "Boids 3D")
        .unwrap();
    session.select(index);
    let initial: Vec<_> = session.sim.states().map(|s| s.position).collect();
    for _ in 0..1200 {
        session.step();
        for state in session.sim.states() {
            assert!(state.position.is_finite() && state.velocity.is_finite());
            assert!(
                (state.position - Vec3::new(0.0, 5.0, 0.0))
                    .abs()
                    .max_element()
                    < 12.1
            );
            assert!(state.velocity.length() < 5.0);
        }
    }
    let moved = session
        .sim
        .states()
        .zip(initial)
        .any(|(state, old)| (state.position - old).abs().min_element() > 0.1);
    assert!(moved);
}
#[test]
fn registry_contains_examples() {
    for name in ["Bouncing Balls", "Boids 3D"] {
        assert!(ALGORITHMS.iter().any(|create| create().name() == name));
    }
}
