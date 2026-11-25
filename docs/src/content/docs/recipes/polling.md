---
title: Wait for a background job
description: Start an async job, poll until it finishes, check the result.
---

```yaml title="critters/admin/flows/export.yml"
name: Export orders
steps:
  - request: exports/create.yml
    expect: 202
    capture:
      jobId: res.body.jobId

  - tool: poll
    request: exports/status.yml
    with:
      jobId: "{{jobId}}"
    until:
      - expression: res.body.state
        operator: in
        value: done, failed
    capture:
      state: res.body.state
      downloadUrl: res.body.url
      rows: res.body.rowCount
    every: 5
    timeout: 300

  - tool: assert
    checks:
      - { expression: "{{state}}", operator: eq, value: done }
      - { expression: "{{rows}}", operator: gt, value: "0" }

  - request: exports/download.yml
    with:
      url: "{{downloadUrl}}"
    expect: 200
```

`until` stops on `done` or `failed`. The `assert` step then fails the flow with the actual state, instead of waiting until the timeout.

## Wait for a server to start

Connection errors count as "not done yet":

```yaml
- tool: poll
  request: health.yml
  until:
    - { expression: res.status, operator: eq, value: "200" }
  every: 1
  timeout: 60
```

## Eventually consistent reads

```yaml
- tool: poll
  request: search/users.yml
  with: { q: "{{email}}" }
  until:
    - { expression: res.body.hits, operator: isNotEmpty }
  timeout: 20
```
