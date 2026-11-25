//! The UI keeps copies of a few things core defines, to show them without a
//! round trip to the backend. These tests fail when a copy drifts.

use cactus_core::http::ASSERTION_OPERATORS;
use cactus_core::layout;

const UI_HTTP: &str = include_str!("../../ui/src/domain/http.ts");
const UI_ENV_FILES: &str = include_str!("../../ui/src/domain/envFiles.ts");
const UI_PATHS: &str = include_str!("../../ui/src/domain/paths.ts");

/// The quoted strings of the TypeScript array literal that follows `marker`.
fn quoted_list<'a>(source: &'a str, marker: &str) -> Vec<&'a str> {
    let start = source.find(marker).unwrap_or_else(|| panic!("{marker} not found")) + marker.len();
    let list = &source[start..source[start..].find(']').map(|end| start + end).unwrap()];
    list.split('"').skip(1).step_by(2).collect()
}

#[test]
fn the_editor_offers_every_operator_core_checks() {
    assert_eq!(quoted_list(UI_HTTP, "export const ASSERTION_OPERATORS = ["), ASSERTION_OPERATORS);
}

#[test]
fn the_set_tool_offers_every_generator_core_has() {
    const UI_GENERATORS: &str = include_str!("../../ui/src/domain/generators.ts");
    assert_eq!(quoted_list(UI_GENERATORS, "export const GENERATORS = ["), cactus_core::tools::GENERATORS);
}

#[test]
fn the_ui_names_the_files_core_uses() {
    let root = std::path::Path::new("");
    let workspace_file = |path: std::path::PathBuf| path.to_string_lossy().replace('\\', "/");
    for expected in [
        layout::COLLECTION_FILE.to_string(),
        format!("{}/{}", layout::WORKSPACE_DIR, cactus_core::secrets::SECRETS_FILE),
        workspace_file(layout::captured_file(root)),
        format!("`{}/${{", layout::ENVIRONMENTS_DIR),
    ] {
        assert!(UI_ENV_FILES.contains(&expected), "ui/src/domain/envFiles.ts doesn't mention {expected}");
    }
    assert!(UI_PATHS.contains(&format!("REQUEST_EXTENSION = \"{}\"", layout::REQUEST_EXTENSION)));
}
