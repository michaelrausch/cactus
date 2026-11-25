---
title: Project layout
description: Every file in a project.
---

## Committed

| Path | |
| --- | --- |
| `opencollection.yml` | Collection: name, default headers and auth, shared variables |
| `environments/<name>.yml` | Environment |
| `<folder>/folder.yml` | Folder name, `seq`, defaults (optional) |
| `**/*.yml` | Requests |
| `critters/<id>/critter.yml` | Critter |
| `critters/<id>/flows/<flow>.yml` | Flow |

## Local (`.cactus/`)

Ignored by git. Sensitive files are [encrypted](/guides/secrets/#encryption).

| Path | |
| --- | --- |
| `secrets.json` | Secret values |
| `captured.json` | Values captured with no critter selected |
| `tokens.json` | OAuth2 tokens fetched with no critter selected |
| `critters/<id>/secrets.json` | Critter secrets |
| `critters/<id>/context.json` | Critter's captured values |
| `critters/<id>/cookies.json` | Critter cookie jar |
| `critters/<id>/tokens.json` | Critter OAuth2 tokens |
| `critters/<id>/runs/<flow>.json` | Last 50 scheduled runs |
| `history/*.json` | Recent responses |
| `state.json` | Active environment and critter |
| `vault` | Password check and authenticator secret |

Deleting `.cactus/` resets local state: secrets, captured values and history.
