pub mod api;
pub mod physics;
pub mod simulation;
pub mod algorithms {
    include!(concat!(env!("OUT_DIR"), "/algorithms.rs"));
}
