---
title: Checks and captures
description: Assert on responses and save values from them.
---

- **Checks** (`runtime.assertions`, **Checks** tab) decide whether a request passes.
- **Captures** (`runtime.actions`, **Outputs** tab) save part of a response as a variable.

## Checks

```yaml
runtime:
  assertions:
    - expression: res.status
      operator: eq
      value: "200"
    - expression: res.body.items
      operator: length
      value: "20"
    - expression: res.headers['content-type']
      operator: startsWith
      value: application/json
    - expression: res.responseTime
      operator: lt
      value: "500"
```

A request passes when all its checks pass. If none of its checks look at `res.status`, a status of 400 or above also fails it.

`value` is read as JSON if it parses, otherwise as text. `{{variables}}` aren't filled in here; to compare against a variable, use a flow [`assert`](/reference/flow-tools/#assert) step.

`disabled: true` turns a check off.

All operators: [Expressions and operators](/reference/operators/).

## Captures

```yaml
runtime:
  actions:
    - type: set-variable
      phase: after-response
      selector:
        expression: res.body.access_token
        method: jsonq
      variable:
        name: token
        scope: runtime
```

Where the value is saved:

| Sending as | Saved to |
| --- | --- |
| No critter | `.cactus/captured.json` |
| A critter | That critter's context, `.cactus/critters/<id>/context.json` |

If the expression matches nothing, the old value stays and the response shows `capture token failed`.

Flow steps can add their own captures:

```yaml
- request: orders/create.yml
  capture:
    orderId: res.body.id
```

## Expressions

| Expression | Selects |
| --- | --- |
| `res.status` | Status code |
| `res.responseTime` | Time in ms |
| `res.headers.etag` | A header (case-insensitive) |
| `res.headers['content-type']` | A header with a dash in its name |
| `res.body.data[0].id` | A path in a JSON body |
| `data.token`, `$.data.token` | Same as `res.body.data.token` |
