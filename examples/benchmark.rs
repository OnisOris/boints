//! Headless physics + algorithm timing; does not measure rendering or browser FPS.
use algorithm_playground::{algorithms::ALGORITHMS, simulation::Simulation};
use std::time::Instant;
fn main() {
    for agents in [100, 200] {
        for selected in 0..ALGORITHMS.len() {
            let mut session = Simulation::new(agents);
            session.select(selected);
            for _ in 0..120 {
                session.step();
            }
            let start = Instant::now();
            for _ in 0..1200 {
                session.step();
            }
            println!(
                "{}: {agents} agents, {:.3} ms/step (1200 steps)",
                session.algorithm.name(),
                start.elapsed().as_secs_f64() * 1000.0 / 1200.0
            );
        }
    }
}
