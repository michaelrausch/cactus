---
title: Flow tools
description: Every tool step and its fields.
---

Text fields can contain `{{variables}}`. Values a tool sets last for the current run only.

## prompt

```yaml
- tool: prompt
  title: New customer
  vars:
    - name: email
      label: Email
    - name: plan
      default: starter
    - name: apiKey
      secret: true
```

| Field | |
| --- | --- |
| `title` | Dialog title |
| `vars[].name` | Variable to set |
| `vars[].label` | Field label; defaults to the name |
| `vars[].default` | Pre-filled value; also used when no one can answer |
| `vars[].secret` | Hidden input; masked in results |

## pick

```yaml
- tool: pick
  title: Which order?
  list: "{{orders}}"
  label: number
  set:
    orderId: id
    email: customer.email
```

| Field | |
| --- | --- |
| `list` | JSON array, or text with one item per line or comma-separated |
| `label` | Path to show for each item; the whole item when left out |
| `set` | `variable: path`; an empty path sets the whole item |

A list with one item is picked automatically. An empty list fails the step.

## display

```yaml
- tool: display
  title: Delete {{count}} users?
  text: "{{baseUrl}}"
  pause: true
```

With `pause: true`, the flow waits for **Continue**.

## set

```yaml
- tool: set
  values:
    email: $randomEmail
    requestId: $uuid
    label: test-{{requestId}}
```

| Generator | Example |
| --- | --- |
| `$uuid` | `9b2f3c1e-4d5a-4b6c-8e7f-0a1b2c3d4e5f` |
| `$timestamp` | `1795651200000` |
| `$isoTime` | `2026-11-26T09:30:00.000Z` |
| `$randomInt` | `482913` |
| `$randomEmail` | `user-3fa9c01b2e@example.com` |

A generator must be the whole value.

## assert

```yaml
- tool: assert
  checks:
    - { expression: "{{count}}", operator: gt, value: "0" }
    - { expression: "{{user}}.email", operator: endsWith, value: "@example.com" }
  each:
    list: "{{items}}"
    checks:
      - { expression: id, operator: isNotEmpty }
```

`expression` is `{{name}}` with an optional path (`.email`, `.length`, `[0].id`), or any text containing variables. Inside `each`, `expression` is a path into each item.

## poll

```yaml
- tool: poll
  request: jobs/status.yml
  with: { jobId: "{{jobId}}" }
  until:
    - { expression: res.body.state, operator: eq, value: done }
  capture:
    resultUrl: res.body.url
  every: 3
  timeout: 120
```

| Field | |
| --- | --- |
| `every` | Seconds between tries; default 2 |
| `timeout` | Seconds before giving up; default 60, max 600 |

Connection errors count as "not done yet". Captures come from the last response.

## wait

```yaml
- tool: wait
  seconds: 10       # max 600
```

## run

```yaml
- tool: run
  flow: login
  critter: admin    # optional; defaults to this flow's critter
```

Runs up to 8 levels deep.

## open

```yaml
- tool: open
  url: "{{dashboardUrl}}/orders/{{orderId}}"
```

Only `http://` and `https://` URLs.

## webhook

```yaml
- tool: webhook
  url: https://hooks.example.com/done
  method: POST      # default
  headers:
    X-Token: "{{hookToken}}"
  body: '{"ok": true}'
```

A JSON body is sent as `application/json` unless `Content-Type` is set. A non-2xx response fails the step.

## slack

```yaml
- tool: slack
  webhook: "{{SLACK_WEBHOOK}}"
  text: Smoke test passed
```

## notify

```yaml
- tool: notify
  title: Export ready
  body: "{{resultUrl}}"
```

## Security tools

`fuzz`, `cert`, `ratelimit` and `loadtest` report findings and metrics. They send as the step's critter, in the run's environment, but keep nothing: no history, captures or cookies, and no logging in again on a 401.

| Field | |
| --- | --- |
| `fail_on` | Lowest severity that fails the step: `never`, `info`, `low`, `medium`, `high` (default), `critical` |
| `checks` | Limits on metrics, by name: `{expression: p95, operator: lt, value: "500"}` |
| `set` | `variable: source` for later steps; see below |

Findings are matched between runs by `rule` and `location`, so a schedule's `findings` alert fires only on new ones.

### set

| Source | Value |
| --- | --- |
| A metric name, e.g. `p95` | The number; empty if not measured |
| `findings` | Every finding, most severe first, as a JSON array |
| `findings.count` | How many |
| `findings.critical` … `findings.info` | How many at that severity |

```yaml
- tool: fuzz
  request: orders/search.yml
  fail_on: never           # so the flow carries on
  set:
    issues: findings
    highs: findings.high
- tool: slack
  webhook: "{{SLACK_WEBHOOK}}"
  text: "{{highs}} high findings"
- tool: assert
  checks:
    - {expression: "{{highs}}", operator: eq, value: "0"}
```

A failed step skips the rest, so a step that fails on its findings sets nothing. A name the tool never reports fails the step before anything is sent.

## fuzz

```yaml
- tool: fuzz
  request: orders/search.yml
  with:                    # variables for the fuzzed request
    orderId: "42"
  targets: [query, body]   # query, path, headers, body; default query, path, body
  payloads: [sqli, mutate] # sqli, xss, traversal, command, overflow, types, mutate; default all
  iterations: 20           # mutated inputs per value; max 200
  seed: 1234               # repeat the same mutations; random when left out
  concurrency: 4           # max 32
  fail_on: medium
```

Replaces one value at a time with each payload. Headers skip `Authorization`, `Cookie`, `Host`, `Connection`, `Transfer-Encoding` and `Content-*`. A name used twice gets an index: `query.id[0]`, `query.id[1]`. A JSON body is fuzzed leaf by leaf, unless it only parses once its variables are filled in. At most 2000 requests.

`mutate` makes inputs from each value's own: boundary numbers (±1, 0, −1, 2³¹, i64 and u64 limits, 1e308), the string truncated, repeated, with characters dropped, flipped or swapped for quotes and separators, odd Unicode, and values of other types (`null`, `[]`, `{"$ne": null}`). JSON numbers stay numbers. The seed used is in the step's output.

Each finding has a `repro`: a curl command that sends the request again, with `Authorization`, cookies, and headers, query pairs and JSON or form fields named like a token, key, secret, session or password masked. `coverage` lists every value tried: requests sent, the statuses that came back (`none` for no response) and how many findings.

| Rule | Severity | When |
| --- | --- | --- |
| `fuzz.path-traversal` | critical | A system file in the response |
| `fuzz.command-injection` | critical | `id` output in the response |
| `fuzz.sql-error` | high | A database error the baseline didn't have |
| `fuzz.reflected-xss` | high | Markup echoed unescaped in an HTML response |
| `fuzz.stack-trace` | medium | A stack trace the baseline didn't have |
| `fuzz.server-error` | medium | A 5xx where the baseline wasn't |
| `fuzz.time-based` | medium | A sleep payload made it 5s slower |
| `fuzz.no-response` | medium | The connection dropped or timed out |

Location: `query.<name>`, `path.<name>`, `header.<name>`, `body.<path>`, `form.<name>`. Metrics: `requests`, `errors`.

## cert

```yaml
- tool: cert
  url: "{{baseUrl}}"       # https URL or host[:port]
  warn_days: 30            # default
  checks:
    - {expression: days_left, operator: gt, value: "14"}
```

Makes its own TLS handshake, and accepts any certificate so a bad one can be inspected. TLS 1.0 and 1.1 aren't tried.

| Rule | Severity | When |
| --- | --- | --- |
| `cert.expired`, `cert.not-yet-valid` | critical | Outside its dates |
| `cert.expiring` | high under 7 days, else medium | Within `warn_days` |
| `cert.untrusted` | high | Chain doesn't lead to a trusted root |
| `cert.hostname` | high | Host not in its names |
| `cert.weak-key` | high | RSA under 2048 bits, EC under 256 |
| `cert.weak-signature` | high | MD5 or SHA-1 |
| `cert.no-tls13` | info | TLS 1.3 not negotiated |

Metrics: `days_left`, `handshake_ms`.

## ratelimit

```yaml
- tool: ratelimit
  request: auth/login.yml
  limit: 10          # most that should get through
  requests: 30       # default 3 × limit, or 100; max 1000
  concurrency: 1     # default; max 100
  reset: true        # once limited, wait Retry-After and try again
```

A 429, or a 503 with `Retry-After`, counts as limited.

| Rule | Severity | When |
| --- | --- | --- |
| `ratelimit.missing` | high | Nothing limited |
| `ratelimit.too-high` | medium | Limited later than `limit` |
| `ratelimit.no-retry-after` | low | A 429 without `Retry-After` or `RateLimit-*` headers |
| `ratelimit.bypass` | low | A 2xx after the first 429 |
| `ratelimit.no-reset` | low | Still limited after waiting (`reset`) |

Waits at most 600 seconds; 60 when `Retry-After` isn't a number. Metrics: `sent`, `allowed`, `limited_at`, `rps`.

## loadtest

```yaml
- tool: loadtest
  request: orders/list.yml
  requests: 500      # default 100; max 10000
  duration: 30       # or send for this many seconds; max 600
  concurrency: 20    # default 10; max 100
  checks:
    - {expression: p95, operator: lt, value: "500"}
    - {expression: error_rate, operator: lt, value: "1"}
```

With `duration` and no `requests`, sends until time is up. Reports metrics only.

| Metric | |
| --- | --- |
| `requests`, `rps` | Sent, and per second |
| `min`, `p50`, `p90`, `p95`, `p99`, `max`, `mean` | Response time, ms |
| `error_rate` | Unanswered or 5xx, % |
| `non2xx` | Responses outside 2xx |
