//! Scenarios: one HTTP request per JSON file.

mod listing;
mod model;
mod store;

pub use listing::{ScenarioEntry, list, walk};
pub use model::{Auth, BodyType, DEFAULT_METHOD, Expect, Scenario, StatusExpect};
pub use store::{create, create_blank, delete, duplicate, load, rename, save};
