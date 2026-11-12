use std::collections::BTreeSet;
use std::path::Path;

use super::store::load;
use crate::Result;
use crate::secrets::{self, SecretFile};

/// Names of the critter's `secret: true` variables with no value in its own
/// secrets or the project's. Reads names only, so it works while secrets
/// are locked: a sealed value counts as present.
pub fn missing_secrets(project: &Path, id: &str) -> Result<Vec<String>> {
    let critter = load(project, id)?;
    let mut present: BTreeSet<String> =
        secrets::names(project, &SecretFile::Critter(id.to_string()))?.into_iter().collect();
    present.extend(secrets::names(project, &SecretFile::Project)?);
    let mut missing: Vec<String> = critter
        .variables
        .iter()
        .filter(|v| v.secret && !v.disabled && !v.name.trim().is_empty())
        .map(|v| v.name.to_string())
        .filter(|name| !present.contains(name))
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
    use crate::env::{self, Env, EnvFile};

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
        env::save(p, &EnvFile::Secrets, &Env::from([("A".into(), "1".into())])).unwrap();
        env::save(p, &EnvFile::CritterSecrets("bob".into()), &Env::from([("B".into(), "2".into())])).unwrap();
        assert_eq!(missing_secrets(p, "bob").unwrap(), ["C"]);
    }
}
