use std::path::Path;

use super::store::load;
use crate::env::Env;
use crate::storage::read_json;
use crate::{Result, layout};

/// Names of the critter's `secret: true` variables with no value in its own
/// secrets or the project's.
pub fn missing_secrets(project: &Path, id: &str) -> Result<Vec<String>> {
    let critter = load(project, id)?;
    let own: Env = read_json(&layout::critter_secrets_file(project, id))?;
    let shared: Env = read_json(&layout::secrets_file(project))?;
    let mut missing: Vec<String> = critter
        .variables
        .iter()
        .filter(|v| v.secret && !v.disabled && !v.name.trim().is_empty())
        .map(|v| v.name.to_string())
        .filter(|name| !own.contains_key(name) && !shared.contains_key(name))
        .collect();
    missing.sort();
    missing.dedup();
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::Variable;
    use crate::critter::{Critter, create};
    use crate::storage::write_json;

    #[test]
    fn secrets_from_either_file_count() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        let secret = |name: &str| Variable { secret: true, ..Variable::new(name, "") };
        let critter = Critter {
            variables: vec![secret("A"), secret("B"), secret("C"), Variable::new("plain", "x")],
            ..Critter::default()
        };
        create(p, "bob", &critter).unwrap();
        assert_eq!(missing_secrets(p, "bob").unwrap(), ["A", "B", "C"]);
        layout::ensure_workspace(p).unwrap();
        write_json(&layout::secrets_file(p), &Env::from([("A".into(), "1".into())])).unwrap();
        std::fs::create_dir_all(layout::critter_workspace_dir(p, "bob")).unwrap();
        write_json(&layout::critter_secrets_file(p, "bob"), &Env::from([("B".into(), "2".into())])).unwrap();
        assert_eq!(missing_secrets(p, "bob").unwrap(), ["C"]);
    }
}
