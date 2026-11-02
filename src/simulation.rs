use crate::{
    algorithms::ALGORITHMS,
    api::{Algorithm, Sim},
    physics::DT,
};

pub struct Simulation {
    pub sim: Sim,
    pub algorithm: Box<dyn Algorithm>,
    pub selected: usize,
    pub playing: bool,
    pub speed: f32,
    accumulator: f32,
}
impl Simulation {
    pub fn new(agents: usize) -> Self {
        let mut this = Self {
            sim: Sim::new(agents),
            algorithm: ALGORITHMS[0](),
            selected: 0,
            playing: true,
            speed: 1.0,
            accumulator: 0.0,
        };
        this.algorithm.setup(&mut this.sim);
        this
    }
    pub fn reset(&mut self) {
        self.sim.clear();
        self.algorithm = ALGORITHMS[self.selected]();
        self.algorithm.setup(&mut self.sim);
        self.accumulator = 0.0;
    }
    pub fn select(&mut self, index: usize) {
        if index < ALGORITHMS.len() {
            self.selected = index;
            self.reset();
        }
    }
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
        self.accumulator = 0.0;
    }
    pub fn step(&mut self) {
        self.algorithm.update(&mut self.sim, DT);
        self.sim.step();
    }
    pub fn single_step(&mut self) {
        self.set_playing(false);
        self.step();
    }
    pub fn frame(&mut self, elapsed: f32) {
        if !self.playing {
            self.accumulator = 0.0;
            return;
        }
        // Discard long stalls (background tab) instead of trying to catch them up.
        if !elapsed.is_finite() || !(0.0..=0.25).contains(&elapsed) {
            self.accumulator = 0.0;
            return;
        }
        self.accumulator = (self.accumulator + elapsed * self.speed).min(8.0 * DT);
        let mut steps = 0;
        while self.accumulator >= DT && steps < 8 {
            self.step();
            self.accumulator -= DT;
            steps += 1;
        }
    }
}
