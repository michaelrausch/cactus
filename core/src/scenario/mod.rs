//! Requests: one OpenCollection `HttpRequest` per YAML file.

mod listing;
mod model;
mod store;

pub(crate) use listing::next_seq;
pub use listing::{ScenarioEntry, is_ignored, list, walk};
pub use model::{
    Action, ActionPhase, ActionVariable, Assertion, Body, DEFAULT_METHOD, FileVariant, FormField, Http, MultipartField,
    Param, ParamKind, PartKind, PartValue, RequestInfo, RequestType, Runtime, Scenario, Script, Selector, SetVariable,
    SetVariableType, Settings, Timeout, TypedBody,
};
pub use store::{create, create_blank, delete, duplicate, load, rename, save};
