---
title: Watch staging, alert Slack
description: A scheduled health flow, plus a CI cron that posts to Slack.
---

## Flow

```yaml title="critters/monitor/flows/staging.yml"
name: Staging
triggers:
  - type: schedule
    every: 5m
    env: staging
alert:
  on: [failure, change]
steps:
  - request: health.yml
    expect: 200
    assert:
      - { expression: res.body.db, operator: eq, value: ok }
      - { expression: res.responseTime, operator: lt, value: "1500" }
    watch:
      - res.body.version
  - request: auth/login.yml
    expect: 200
  - request: products/list.yml
    expect: 200
    assert:
      - { expression: res.body.items, operator: isNotEmpty }
```

While the app is open, you get an alert when staging starts failing, when it recovers, and when `version` changes.

## Slack via CI

App alerts go only to you. To post to a channel around the clock, run the same flow on a cron:

```yaml title=".github/workflows/staging-watch.yml"
name: Watch staging
on:
  schedule:
    - cron: "*/10 * * * *"

jobs:
  watch:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install --git https://github.com/michaelrausch/cactus cactus-cli --locked
      - run: cactus-cli flow api-tests monitor staging -e staging
      - if: failure()
        run: >
          curl -sf -X POST -H 'content-type: application/json'
          -d '{"text":"Staging check failed: ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}"}'
          "${{ secrets.SLACK_WEBHOOK }}"
```

A `slack` step inside a flow posts only if every earlier step passed. That suits success messages, such as "deploy verified".

## Detect changes in another API

```yaml
name: Partner API drift
triggers:
  - { type: schedule, every: 1h }
alert:
  on: [change]
steps:
  - request: partner/rates.yml
    watch:
      - res.headers.x-api-version
      - res.body.schemaVersion
```
