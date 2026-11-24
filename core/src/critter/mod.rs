//! Critters: personas with their own variables, secrets and captured
//! context, and flows that chain requests as them.

mod model;
mod secrets;
mod store;

pub use model::{
    AlertOn, AlertRule, ContextEntry, Critter, Flow, FlowStep, Interval, Species, Trigger, validate_critter_id,
    validate_flow_id,
};
pub use secrets::missing_secrets;
pub use store::{
    CritterSummary, FlowParse, FlowSummary, active, create, create_flow, delete, delete_flow, duplicate,
    enabled_schedules, exists, flow_yaml, list, list_flows, load, load_flow, load_flow_text, parse_flow, rename,
    rename_flow, save, save_flow, save_flow_text, schedule_key, set_active, set_schedule_enabled,
};
