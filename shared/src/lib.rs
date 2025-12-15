//! Shared, deterministic gameplay logic intended to run identically on client and server.
//!
//! Public API policy
//! -----------------
//! Expose only what server and client need to:
//! - build an immutable collision world (statics + accelerator), and
//! - run deterministic movement (`step_movement`).

pub mod collision;
mod motion;

pub mod movement;

pub use movement::{StepMovementResult, step_movement};
