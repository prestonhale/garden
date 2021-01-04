use garden::world;
use rand_core::SeedableRng;


// This is a benchmark
// 
// Running a traditional benchmark (e.g. via Criterion) ends up hiding all of 
// its function calls behind dyld_start.
// Run: `cargo instruments --example world_update --open`
fn main() {
    let mut my_world = world::World::default();
    let mut randomizer = rand_pcg::Pcg32::from_seed(*b"somebody once to");
    for _ in 0..1000 {
        my_world.update(&mut randomizer);
    };
}