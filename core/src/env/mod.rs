//! Variables: layered env files and `{{name}}` substitution.

mod capture;
mod layers;
mod template;

pub(crate) use capture::ensure_critter_workspace;
pub use capture::{Context, EDITED, clear_context, load_context, merge_captured, merge_context, restore_context};
pub use layers::{
    Env, EnvFile, RequestScope, Scope, VarSource, active, delete_named, list_named, load, rename_named, resolve,
    resolve_for_request, save, set_active, sources_for_request, validate_env_name,
};
pub use template::Substituter;
