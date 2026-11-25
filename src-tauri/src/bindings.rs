//! The typed command/event surface, and its generated TypeScript bindings.

use tauri_specta::{Builder, collect_commands, collect_events};

use crate::commands::{critter, env, flow, git, import, mcp, project, request, scenario, schedule, update};
use crate::events::{
    ContextChanged, FlowAskClosed, FlowAsked, FlowDisplayed, FlowNotified, FlowProgress, FlowStepStarted, FsChanged,
    HistoryChanged, MatrixProgress, MonitorAlert, RunProgress, ScheduledRunFinished,
};

#[cfg(debug_assertions)]
const TYPESCRIPT_BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ui/src/api/bindings.ts");

#[allow(clippy::too_many_lines)] // One line per command and event.
pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            project::open_project,
            project::list_scenarios,
            project::recent_projects,
            project::vault_status,
            project::unlock_vault,
            project::wipe_vault,
            project::totp_enabled,
            project::start_totp,
            project::enable_totp,
            project::disable_totp,
            project::set_vault_password,
            scenario::load_scenario,
            scenario::save_scenario,
            scenario::create_scenario,
            scenario::duplicate_scenario,
            scenario::delete_scenario,
            scenario::rename_scenario,
            scenario::retitle_scenario,
            scenario::create_folder,
            scenario::list_folders,
            scenario::load_history,
            env::list_envs,
            env::active_env,
            env::set_active_env,
            env::load_env,
            env::save_env,
            env::rename_env,
            env::delete_env,
            env::variable_sources,
            env::base_variable_sources,
            critter::list_critters,
            critter::load_critter,
            critter::save_critter,
            critter::create_critter,
            critter::rename_critter,
            critter::duplicate_critter,
            critter::delete_critter,
            critter::active_critter,
            critter::set_active_critter,
            critter::load_context,
            critter::restore_context,
            critter::clear_context,
            critter::load_cookies,
            critter::remove_cookies,
            flow::list_flows,
            flow::load_flow,
            flow::save_flow,
            flow::create_flow,
            flow::rename_flow,
            flow::duplicate_flow,
            flow::delete_flow,
            flow::load_flow_text,
            flow::save_flow_text,
            flow::parse_flow,
            flow::flow_to_yaml,
            flow::flow_variable_sources,
            flow::run_flow,
            flow::answer_flow_ask,
            flow::run_matrix,
            schedule::set_schedules_paused,
            schedule::schedule_states,
            schedule::load_run_log,
            schedule::run_scheduled_now,
            git::git_status,
            git::git_init,
            git::git_stage,
            git::git_unstage,
            git::git_stage_all,
            git::git_unstage_all,
            git::git_discard,
            git::git_discard_all,
            git::git_diff,
            git::git_commit,
            git::git_configured_identity,
            git::git_branches,
            git::git_create_branch,
            git::git_switch_branch,
            git::git_delete_branch,
            import::import_file,
            request::send_request,
            request::run_folder,
            request::cancel_request,
            request::oauth2_token_status,
            request::get_oauth2_token,
            request::clear_oauth2_token,
            mcp::mcp_status,
            mcp::set_mcp_settings,
            mcp::regenerate_mcp_token,
            update::check_for_update,
            update::open_website,
        ])
        .events(collect_events![
            FsChanged,
            RunProgress,
            FlowStepStarted,
            FlowProgress,
            FlowAsked,
            FlowAskClosed,
            FlowDisplayed,
            FlowNotified,
            MatrixProgress,
            ScheduledRunFinished,
            MonitorAlert,
            ContextChanged,
            HistoryChanged
        ])
}

/// Regenerate `ui/src/api/bindings.ts`; debug builds only, so the UI's types
/// always match the commands it is talking to.
///
/// # Panics
/// If the types can't be exported, which is a programming error.
#[cfg(debug_assertions)]
pub fn export_typescript(builder: &Builder<tauri::Wry>) {
    builder.export(specta_typescript::Typescript::default(), TYPESCRIPT_BINDINGS).expect("TypeScript bindings export");
}

#[cfg(test)]
mod tests {
    /// `cargo test -p cactus` regenerates the bindings without starting the app.
    #[test]
    fn export_typescript_bindings() {
        super::export_typescript(&super::builder());
    }
}
