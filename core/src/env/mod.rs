//! Variables: layered env files and `{{name}}` substitution.

mod capture;
mod layers;
mod template;

pub use capture::merge_captured;
pub use layers::{Env, EnvFile, active, list_named, load, resolve, save, set_active, validate_env_name};
pub use template::Substituter;
