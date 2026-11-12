use std::fs;
use std::path::Path;

use super::store::fail_point::fail_after;
use super::*;
use crate::Error;
use crate::env::{self, Env, EnvFile, Scope};

fn env(pairs: &[(&str, &str)]) -> Env {
    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

fn pw(p: &str) -> SecretString {
    p.to_string().into()
}

fn raw(p: &Path, rel: &str) -> Env {
    serde_json::from_slice(&fs::read(p.join(rel)).unwrap_or_else(|_| b"{}".to_vec())).unwrap()
}

const PROJECT: &str = ".cactus/secrets.json";
const BOB: &str = ".cactus/critters/bob/secrets.json";

/// A project with a project secret and critter Bob's.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    crate::critter::create(p, "bob", &crate::critter::Critter::default()).unwrap();
    env::save(p, &EnvFile::Secrets, &env(&[("API_KEY", "k1"), ("shared", "s")])).unwrap();
    env::save(p, &EnvFile::CritterSecrets("bob".into()), &env(&[("token", "t1")])).unwrap();
    dir
}

/// Every file's values opened with the held keys.
fn values(p: &Path) -> Vec<Env> {
    SecretFile::all(p).iter().map(|f| read(p, f).unwrap()).collect()
}

fn all_sealed(e: &Env) -> bool {
    e.values().all(|v| is_sealed(v))
}

#[test]
fn off_behaves_as_before() {
    let dir = project();
    let p = dir.path();
    assert_eq!(status(p).unwrap(), SecretsStatus::Off);
    assert_eq!(raw(p, PROJECT), env(&[("API_KEY", "k1"), ("shared", "s")]));
    assert_eq!(fs::read_to_string(p.join(".cactus/state.json")).ok().filter(|s| s.contains("secrets")), None);
}

#[test]
fn enable_seals_every_file_and_leaves_it_unlocked() {
    let dir = project();
    let p = dir.path();
    let before = values(p);
    assert!(matches!(enable(p, &pw("short"), Some(15)), Err(Error::PasswordTooShort)));
    enable(p, &pw("password1"), Some(15)).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Unlocked);
    assert!(all_sealed(&raw(p, PROJECT)) && all_sealed(&raw(p, BOB)));
    assert_eq!(raw(p, PROJECT).keys().collect::<Vec<_>>(), ["API_KEY", "shared"], "names stay readable");
    assert_eq!(values(p), before);
    assert!(matches!(enable(p, &pw("password1"), None), Err(Error::EncryptionAlreadyOn)));

    // A critter created after enabling gets sealed values.
    crate::critter::create(p, "alice", &crate::critter::Critter::default()).unwrap();
    env::save(p, &EnvFile::CritterSecrets("alice".into()), &env(&[("x", "1")])).unwrap();
    assert!(all_sealed(&raw(p, ".cactus/critters/alice/secrets.json")));
}

#[test]
fn locked_fails_values_but_not_names() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    lock(p);
    assert_eq!(status(p).unwrap(), SecretsStatus::Locked);
    assert!(matches!(env::load(p, &EnvFile::Secrets), Err(Error::SecretsLocked)));
    let scope = Scope::default().with_critter(Some("bob".into()));
    assert!(matches!(env::resolve_for_request(p, &scope, "", &[]), Err(Error::SecretsLocked)));
    let sources = env::sources_for_request(p, &scope, "", &[]).unwrap();
    assert!(sources.contains_key("API_KEY") && sources.contains_key("token"));
    assert_eq!(names(p, &SecretFile::Critter("bob".into())).unwrap(), ["token"]);
    assert!(locked_for(p, &[SecretFile::Project]));
    assert!(matches!(env::save(p, &EnvFile::Secrets, &env(&[("a", "b")])), Err(Error::SecretsLocked)));

    assert!(matches!(unlock(p, &pw("wrong-password")), Err(Error::WrongPassword)));
    unlock(p, &pw("password1")).unwrap();
    assert_eq!(env::resolve_for_request(p, &scope, "", &[]).unwrap().vars["token"], "t1");
    assert!(!locked_for(p, &[SecretFile::Project]));
}

#[test]
fn a_file_with_only_plain_values_never_needs_the_password() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    lock(p);
    fs::write(p.join(PROJECT), r#"{"plain": "v"}"#).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Locked, "still on: state.json says so");
    assert_eq!(env::load(p, &EnvFile::Secrets).unwrap(), env(&[("plain", "v")]));
}

#[test]
fn plain_values_while_on_are_sealed_on_unlock() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    lock(p);
    // As an older cactus or a hand edit would leave it.
    let mut file = raw(p, PROJECT);
    file.insert("NEW".into(), "plain".into());
    fs::write(p.join(PROJECT), serde_json::to_string(&file).unwrap()).unwrap();
    assert_eq!(unlock(p, &pw("password1")).unwrap().sealed_plain_values, 1);
    assert!(all_sealed(&raw(p, PROJECT)));
    assert_eq!(read(p, &SecretFile::Project).unwrap()["NEW"], "plain");
}

#[test]
fn an_older_build_saving_sealed_values_back_loses_nothing() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    let stored = fs::read(p.join(PROJECT)).unwrap();
    let parsed: Env = serde_json::from_slice(&stored).unwrap();
    fs::write(p.join(PROJECT), serde_json::to_vec_pretty(&parsed).unwrap()).unwrap();
    lock(p);
    unlock(p, &pw("password1")).unwrap();
    assert_eq!(read(p, &SecretFile::Project).unwrap()["API_KEY"], "k1");
}

#[test]
fn disable_needs_the_password_and_restores_plain_text() {
    let dir = project();
    let p = dir.path();
    let before = values(p);
    enable(p, &pw("password1"), None).unwrap();
    let sealed = fs::read(p.join(PROJECT)).unwrap();
    assert!(matches!(disable(p, &pw("wrong-password")), Err(Error::WrongPassword)));
    assert_eq!(fs::read(p.join(PROJECT)).unwrap(), sealed, "a wrong password changes nothing");
    disable(p, &pw("password1")).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Off);
    assert_eq!(raw(p, PROJECT), before[0]);
    assert_eq!(raw(p, BOB), before[1]);
}

#[test]
fn change_password_reseals_under_the_new_one() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    assert!(matches!(change_password(p, &pw("password1"), &pw("short")), Err(Error::PasswordTooShort)));
    assert!(matches!(change_password(p, &pw("nope-nope"), &pw("password2")), Err(Error::WrongPassword)));
    change_password(p, &pw("password1"), &pw("password2")).unwrap();
    lock(p);
    assert!(matches!(unlock(p, &pw("password1")), Err(Error::WrongPassword)));
    unlock(p, &pw("password2")).unwrap();
    assert_eq!(read(p, &SecretFile::Project).unwrap()["API_KEY"], "k1");
}

#[test]
fn an_interrupted_password_change_recovers_with_both_passwords() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    fail_after(Some(1));
    assert!(change_password(p, &pw("password1"), &pw("password2")).is_err());
    fail_after(None);
    lock(p);
    unlock(p, &pw("password2")).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Locked, "Bob's values are still under the old password");
    unlock(p, &pw("password1")).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Unlocked);
    lock(p);
    unlock(p, &pw("password1")).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Unlocked, "every value is under one password again");
    assert_eq!(read(p, &SecretFile::Critter("bob".into())).unwrap()["token"], "t1");
}

#[test]
fn reset_drops_sealed_values_and_keeps_plain_ones() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    lock(p);
    let mut file = raw(p, PROJECT);
    file.insert("PLAIN".into(), "kept".into());
    fs::write(p.join(PROJECT), serde_json::to_string(&file).unwrap()).unwrap();
    reset(p).unwrap();
    assert_eq!(status(p).unwrap(), SecretsStatus::Off);
    assert_eq!(raw(p, PROJECT), env(&[("PLAIN", "kept")]));
    assert!(raw(p, BOB).is_empty());
}

#[test]
fn every_crash_point_leaves_secrets_readable() {
    let original =
        |p: &Path| (read(p, &SecretFile::Project).unwrap(), read(p, &SecretFile::Critter("bob".into())).unwrap());
    for failing_write in 0..2 {
        // Enable.
        let dir = project();
        let p = dir.path();
        let before = original(p);
        fail_after(Some(failing_write));
        assert!(enable(p, &pw("password1"), None).is_err());
        fail_after(None);
        assert_eq!(original(p), before);
        lock(p);
        unlock(p, &pw("password1")).unwrap();
        assert!(all_sealed(&raw(p, PROJECT)) && all_sealed(&raw(p, BOB)));
        assert_eq!(original(p), before);

        // Disable.
        fail_after(Some(failing_write));
        assert!(disable(p, &pw("password1")).is_err());
        fail_after(None);
        assert_eq!(original(p), before);
        disable(p, &pw("password1")).unwrap();
        assert_eq!(original(p), before);

        // Reset: sealed values are gone or still openable, never stranded.
        enable(p, &pw("password1"), None).unwrap();
        fail_after(Some(failing_write));
        assert!(reset(p).is_err());
        fail_after(None);
        assert_eq!(status(p).unwrap(), SecretsStatus::Unlocked);
        for file in SecretFile::all(p) {
            read(p, &file).unwrap();
        }
    }
}

#[test]
fn malformed_and_newer_values_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    fs::create_dir_all(p.join(".cactus")).unwrap();
    fs::write(p.join(PROJECT), r#"{"A": "enc:v9:x:y"}"#).unwrap();
    assert!(matches!(env::load(p, &EnvFile::Secrets), Err(Error::UnsupportedSecretFormat(n)) if n == "A"));
    fs::write(p.join(PROJECT), r#"{"A": "enc:v1:nosalt"}"#).unwrap();
    assert!(matches!(env::load(p, &EnvFile::Secrets), Err(Error::SecretCorrupt(_))));
}

#[test]
fn a_sealed_value_without_state_still_counts_as_on() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    fs::remove_file(p.join(".cactus/state.json")).unwrap();
    lock(p);
    assert_eq!(status(p).unwrap(), SecretsStatus::Locked);
    assert!(matches!(env::save(p, &EnvFile::Secrets, &env(&[("a", "b")])), Err(Error::SecretsLocked)));
}

#[test]
fn renaming_a_critter_keeps_its_secrets_openable() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    crate::critter::rename(p, "bob", "robert").unwrap();
    lock(p);
    unlock(p, &pw("password1")).unwrap();
    assert_eq!(read(p, &SecretFile::Critter("robert".into())).unwrap()["token"], "t1");
}

#[test]
fn missing_secrets_works_while_locked() {
    let dir = project();
    let p = dir.path();
    let secret =
        |name: &str| crate::collection::Variable { secret: true, ..crate::collection::Variable::new(name, "") };
    let bob = crate::critter::Critter {
        variables: vec![secret("token"), secret("API_KEY"), secret("gone")],
        ..crate::critter::Critter::default()
    };
    crate::critter::save(p, "bob", &bob).unwrap();
    enable(p, &pw("password1"), None).unwrap();
    lock(p);
    assert_eq!(crate::critter::missing_secrets(p, "bob").unwrap(), ["gone"]);
}

#[test]
fn unlocking_with_nothing_sealed_sets_the_password() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    enable(p, &pw("password1"), Some(5)).unwrap();
    lock(p);
    let info = info(p).unwrap();
    assert_eq!((info.status, info.auto_lock_minutes, info.has_sealed_values), (SecretsStatus::Locked, Some(5), false));
    unlock(p, &pw("password9")).unwrap();
    env::save(p, &EnvFile::Secrets, &env(&[("a", "b")])).unwrap();
    lock(p);
    assert!(matches!(unlock(p, &pw("password1")), Err(Error::WrongPassword)));
    unlock(p, &pw("password9")).unwrap();
    set_auto_lock(p, None).unwrap();
    assert_eq!(super::info(p).unwrap().auto_lock_minutes, None);
}

#[test]
fn read_only_unlock_writes_nothing() {
    let dir = project();
    let p = dir.path();
    enable(p, &pw("password1"), None).unwrap();
    lock(p);
    let mut file = raw(p, PROJECT);
    file.insert("NEW".into(), "plain".into());
    let bytes = serde_json::to_vec(&file).unwrap();
    fs::write(p.join(PROJECT), &bytes).unwrap();
    assert!(matches!(unlock_read_only(p, &pw("wrong-password")), Err(Error::WrongPassword)));
    unlock_read_only(p, &pw("password1")).unwrap();
    assert_eq!(read(p, &SecretFile::Project).unwrap()["API_KEY"], "k1");
    assert_eq!(fs::read(p.join(PROJECT)).unwrap(), bytes);
}
