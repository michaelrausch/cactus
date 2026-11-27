---
title: CLI
description: cactus-cli commands, flags, exit codes and output.
---

```
cactus-cli <command> <project> [arguments] [flags]
```

`<project>` is the folder containing `opencollection.yml`. Flags can go anywhere after the command.

## Exit codes

| | |
| --- | --- |
| `0` | Passed |
| `1` | A check, status or step failed |
| `2` | Couldn't run |

## run

Sends every request in the project, or in one folder, in tree order.

```sh
cactus-cli run <project> [folder] [-e <env>] [--as <critter>] [--json]
```

```sh
cactus-cli run .
cactus-cli run . orders -e staging
cactus-cli run . --as viewer --json > results.json
```

## send

Sends one request. The body goes to stdout; the status line and warnings go to stderr.

```sh
cactus-cli send <project> <request> [-e <env>] [--as <critter>]
```

```sh
cactus-cli send . users/me.yml --as admin | jq .email
```

The exit code reflects the request's checks.

## flow

Runs one flow, or every flow of every bot.

```sh
cactus-cli flow <project> <critter> <flow> [flags]
cactus-cli flow <project> --all [flags]
```

| Flag | |
| --- | --- |
| `--all` | Every flow of every bot |
| `-e`, `--env <NAME>` | Environment |
| `--json` | JSON output |
| `--junit <FILE>` | Also write a JUnit XML report |
| `--var <NAME=VALUE>` | Answer a prompt or pick; repeatable |

```sh
cactus-cli flow . admin smoke -e staging
cactus-cli flow . --all --junit flows.xml
cactus-cli flow . support refund --var email=jo@example.com
```

### Prompts

| Situation | Behaviour |
| --- | --- |
| `--var` given | Answers the field with that name |
| Terminal | Asks for any unanswered fields; secret input is hidden |
| No terminal | Uses the field's `default`; with no default, the step fails |
| `pick` | Answered by a `--var` matching a value the item would set |
| `display` with `pause` | Doesn't wait without a terminal |
| `open` | Prints the URL; opens a browser only at a terminal |
| `notify` | Prints to stderr |

## import

```sh
cactus-cli import <project> <file>
```

Postman v2.x, OpenAPI 3, Swagger 2. See [Importing](/guides/importing/).

## Common flags

| Flag | |
| --- | --- |
| `-e`, `--env <NAME>` | Use this environment instead of the active one |
| `--as <CRITTER>` | Send as this critter (`run`, `send`) |

## Environment variables

| | |
| --- | --- |
| `CACTUS_PASSWORD` | Vault password |
| `CACTUS_TOTP_CODE` | Authenticator code |

## Output

```
admin › smoke
✓ 200    184ms  1. Login  auth/login.yml
✗ 500     92ms  2. Create order  orders/create.yml
      res.status: expected eq 201, got 500
-   -        -  3. Get order  orders/get.yml  skipped

0/1 flow passed
```
