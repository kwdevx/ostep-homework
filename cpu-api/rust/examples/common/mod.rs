// Shared model definition for the model_writer/model_worker demo.
// Mirrors mmap-sync's own examples/common.rs -- `mod common;` in each
// example binary pulls this in without it becoming an example itself
// (Cargo only auto-registers examples/<name>.rs and
// examples/<name>/main.rs, not examples/common/mod.rs).
//
// Each binary only uses half of this module (writer never scores,
// worker never stamps a timestamp), so allow the other half's items
// to go unused in either compilation rather than split the module.
#![allow(dead_code)]

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

pub const SHM_PATH: &str = "/tmp/ostep_cpu_api_model";

/// Stand-in for a live risk/bot-score model: a tiny linear model plus
/// a sigmoid, the same shape (just much smaller) as what Cloudflare's
/// blog post describes serving out of mmap-sync.
#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive_attr(derive(CheckBytes))]
pub struct ScoreModel {
    pub generation: u32,
    pub bias: f32,
    pub weights: [f32; 4],
    pub updated_at_ms: u64,
}

/// A fixed fake request's feature vector. In production these would
/// come from the request itself (headers, IP reputation, rate, ...);
/// here it's constant so every score change we see is purely the
/// model changing under the worker's feet, live, mid-poll-loop.
pub const REQUEST_FEATURES: [f32; 4] = [1.0, 0.4, -0.3, 2.5];

pub fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

pub fn score(model: &ArchivedScoreModel, features: &[f32; 4]) -> f32 {
    let raw: f32 = model.bias
        + model
            .weights
            .iter()
            .zip(features.iter())
            .map(|(w, x)| w * x)
            .sum::<f32>();
    sigmoid(raw)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
