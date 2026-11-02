#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
probe="src/algorithms/registry_probe_$$.rs"
test_name="registry_probe_$$"
test_file="tests/$test_name.rs"
cleanup() { rm -f "$probe" "$test_file"; }
trap cleanup EXIT
cat > "$probe" <<'RS'
use crate::api::{Algorithm, Sim};
struct Probe;
impl Algorithm for Probe {
    fn name(&self) -> &'static str { "registry smoke probe" }
    fn setup(&mut self, _: &mut Sim) {}
    fn update(&mut self, _: &mut Sim, _: f32) {}
}
pub fn create() -> Box<dyn Algorithm> { Box::new(Probe) }
RS
cat > "$test_file" <<'RS'
#[test]
fn new_algorithm_discovered() {
    assert!(algorithm_playground::algorithms::ALGORITHMS.iter()
        .any(|create| create().name() == "registry smoke probe"));
}
RS
cargo test --locked --test "$test_name"
rm -f "$probe"
cat > "$test_file" <<'RS'
#[test]
fn removed_algorithm_disappears() {
    assert!(!algorithm_playground::algorithms::ALGORITHMS.iter()
        .any(|create| create().name() == "registry smoke probe"));
}
RS
cargo test --locked --test "$test_name"
