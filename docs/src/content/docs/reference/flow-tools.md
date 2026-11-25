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
