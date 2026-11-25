---
title: Schedules and alerts
description: Run a flow on a timer and get alerts when it fails, recovers, or a value changes.
---

```yaml title="critters/viewer/flows/health.yml"
name: Health
triggers:
  - type: schedule
    every: 5m             # 30s minimum; s, m, h; a bare number is seconds
    env: staging          # optional; the active environment when left out
alert:
  on: [failure, change]   # default: [failure]
steps:
  - request: health.yml
    expect: 200
    watch:
      - res.body.version
```

- Schedules run while the app has the project open, starting once the file is saved.
- A run never overlaps the one before it.
- Runs missed while the computer slept are skipped.
- **Pause schedules** in the critter picker stops them all.

## Alerts

| `on` | Alerts when |
| --- | --- |
| `failure` | The flow starts failing, and again when it recovers |
| `change` | A `watch` value differs from the previous run |

Alerts fire only when the state changes. A flow that keeps failing alerts once.

When the app is in focus, an alert shows as a toast. Otherwise it's a system notification, and the app shows how many alerts came in.

## Run log

The last 50 runs of each flow are kept in `.cactus/critters/<id>/runs/`. **Run now** runs the flow immediately, recording and alerting like a scheduled run.

## Without the app

Run the same flow from a CI cron job. See [Watch staging](/recipes/monitoring/).
