//! Thin adapters from Tauri commands to `cactus_core`. Arguments are owned
//! because Tauri deserialises them per call.
#![allow(clippy::needless_pass_by_value)]

pub mod critter;
pub mod env;
pub mod flow;
pub mod git;
pub mod import;
pub mod project;
pub mod request;
pub mod scenario;
pub mod schedule;
pub mod secrets;
