// Stand-in for one of many request-handling worker processes. Started
// completely independently of model_writer (no shared parent, no
// fork()) -- just points a Synchronizer at the same SHM_PATH and
// starts reading. Scores one fixed fake request repeatedly, so every
// printed change in score is the live model swapping underneath it,
// never a restart, never a lock held against the writer.
mod common;

use common::{score, ScoreModel, REQUEST_FEATURES, SHM_PATH};
use mmap_sync::synchronizer::Synchronizer;
use std::thread::sleep;
use std::time::Duration;

const POLLS: u32 = 60;

fn main() {
    let worker_id = std::env::args().nth(1).unwrap_or_else(|| "0".to_string());

    let mut synchronizer = Synchronizer::new(SHM_PATH.as_ref());
    let mut last_generation: Option<u32> = None;

    for _ in 0..POLLS {
        if let Ok(model) = unsafe { synchronizer.read::<ScoreModel>(false) } {
            if last_generation != Some(model.generation) {
                let risk = score(&model, &REQUEST_FEATURES);
                println!(
                    "[worker {worker_id}] generation={} risk_score={risk:.3} bias={:.3}",
                    model.generation, model.bias
                );
                last_generation = Some(model.generation);
            }
        }
        sleep(Duration::from_millis(100));
    }

    println!("[worker {worker_id}] done");
}
