---
title: Secrets and the vault
description: Keep secret values out of git, encrypted, optionally behind a password.
---

## Secrets

```yaml
variables:
  - name: apiKey
    secret: true
```

Enter the value under **Variables** (<kbd>Ctrl</kbd>+<kbd>E</kbd>). It's stored in `.cactus/secrets.json`, or `.cactus/critters/<id>/secrets.json` for a critter. Secret values are masked in the app, the CLI, and MCP results.

## Encryption

Secrets, captured values, cookies, OAuth2 tokens and history are encrypted (XChaCha20-Poly1305).

| | Key |
| --- | --- |
| No password | Built into cactus. This hides the values but doesn't protect them. |
| Password | Derived from the password (Argon2id). |

## Password

Set it under **Variables → Encryption**. The app asks for it when the project opens. A forgotten password can't be recovered; the lock screen offers to wipe secrets and captured values.

## Authenticator

**Encryption → Authenticator → Set up**. Unlocking then also needs a TOTP code. The code is checked on your machine, so it isn't a true second factor.

## CLI

| Variable | |
| --- | --- |
| `CACTUS_PASSWORD` | Vault password |
| `CACTUS_TOTP_CODE` | Authenticator code |

At a terminal, the CLI prompts for whichever is missing.

## CI

CI has no `.cactus/`. Pass secrets to a `prompt` step with `--var`:

```yaml
steps:
  - tool: prompt
    vars:
      - name: password
        secret: true
  - request: auth/login.yml
```

```sh
cactus-cli flow . ci smoke --var "password=$CI_PASSWORD"
```
