//! Variables: layered env files and `{{name}}` substitution.

mod capture;
mod layers;
mod template;

pub use capture::merge_captured;
pub use layers::{
    Env, EnvFile, RequestScope, active, delete_named, list_named, load, names_in_scope, rename_named, resolve,
    resolve_for_request, save, set_active, validate_env_name,
};
pub use template::Substituter;
