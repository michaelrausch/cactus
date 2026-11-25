//! Critters: personas with their own variables, secrets and captured
//! context, and flows that chain requests as them.

mod model;
mod secrets;
mod store;

pub use model::{
    AlertOn, AlertRule, ContextEntry, Critter, Flow, FlowStep, Interval, Schedule, Species, Step, Trigger,
    validate_critter_id, validate_flow_id,
};
pub use secrets::missing_secrets;
pub use store::{
    CritterSummary, FlowKey, FlowParse, FlowSummary, active, create, create_flow, delete, delete_flow, duplicate,
    duplicate_flow, exists, flow_keys, flow_yaml, follow_moved_request, list, list_flows, load, load_flow,
    load_flow_text, parse_flow, rename, rename_flow, require, save, save_flow, save_flow_text, set_active,
};
