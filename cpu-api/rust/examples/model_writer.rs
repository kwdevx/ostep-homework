// Stand-in for Cloudflare's model-retraining/config service: the ONE
// process allowed to write. Runs standalone -- no fork() involved --
// to make the point that mmap-sync's sharing is by *named file*, not
// by common ancestry. Any number of unrelated worker processes can
// attach to SHM_PATH and read whatever this publishes.
mod common;

use common::{now_ms, ScoreModel, SHM_PATH};
use mmap_sync::synchronizer::Synchronizer;
use std::thread::sleep;
use std::time::Duration;

const GENERATIONS: u32 = 10;

fn main() {
    let mut synchronizer = Synchronizer::new(SHM_PATH.as_ref());

    for generation in 0..GENERATIONS {
        // Simulate a model drifting as it's periodically retrained on
        // fresh traffic -- e.g. bias creeping up as attack traffic
        // rises. Weights stay fixed; only bias moves, so every score
        // change a worker reports is directly attributable to this.
        let model = ScoreModel {
            generation,
            bias: -1.0 + 0.15 * generation as f32,
            weights: [0.8, -0.5, 0.3, 0.2],
            updated_at_ms: now_ms(),
        };

        let (written, reset) = synchronizer
            .write(&model, Duration::from_secs(1))
            .expect("write failed");
        println!(
            "[writer] published generation={} bias={:.3} ({} bytes, reset={})",
            model.generation, model.bias, written, reset
        );

        sleep(Duration::from_millis(500));
    }

    println!("[writer] done");
}
