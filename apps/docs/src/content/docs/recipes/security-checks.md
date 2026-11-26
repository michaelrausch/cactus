---
title: Security checks in CI
description: Fuzz, rate limit, certificate and load checks that fail the build.
---

## Flow

```yaml title="critters/ci/flows/security.yml"
name: Security
steps:
  - tool: run
    flow: login
  - tool: cert
    url: "{{baseUrl}}"
    warn_days: 21
  - tool: ratelimit
    request: auth/login.yml
    limit: 10
  - tool: fuzz
    request: orders/search.yml
    fail_on: medium
  - tool: loadtest
    request: orders/list.yml
    requests: 300
    concurrency: 20
    checks:
      - {expression: p95, operator: lt, value: "800"}
      - {expression: error_rate, operator: lt, value: "1"}
```

A failed step skips the rest. Put the checks you care about most first.

To report before failing, set `fail_on: never` and hand the results on:

```yaml
  - tool: fuzz
    request: orders/search.yml
    fail_on: never
    set: {highs: findings.high}
  - tool: loadtest
    request: orders/list.yml
    set: {latency: p95}
  - tool: slack
    webhook: "{{SLACK_WEBHOOK}}"
    text: "Fuzz: {{highs}} high · p95 {{latency}}ms"
  - tool: assert
    checks:
      - {expression: "{{highs}}", operator: eq, value: "0"}
```

## Run

```sh
cactus-cli flow api-tests ci security -e staging --junit security.xml
```

Findings print under their step, most severe first; metrics on one line. In JUnit, a failing step's findings are in its failure message.

| Exit | |
| --- | --- |
| `0` | No findings at `fail_on` or above, every check passed |
| `1` | A step failed |

## On a schedule

```yaml title="critters/ci/flows/security.yml"
triggers:
  - type: schedule
    every: 1h
    env: staging
alert:
  on: [failure, findings]
```

While the app is open, `findings` alerts you only on findings the previous run didn't have.

Point `ratelimit`, `fuzz` and `loadtest` at a staging server, not production.
