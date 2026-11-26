---
title: Flow file
description: Every key in a flow file and a critter file.
---

```yaml title="critters/<critter>/flows/<flow>.yml"
name: Staging health

triggers:
  - type: schedule
    every: 5m
    env: staging

alert:
  on: [failure, change]

steps:
  - request: auth/login.yml
    with: { user: bob }
    capture: { token: res.body.access_token }
    expect: 200
    assert:
      - { expression: res.body.ok, operator: eq, value: "true" }
    watch:
      - res.body.version

  - tool: wait
    seconds: 5
```

The flow's id is its file name. Ids use letters, digits, `-` and `_`.

## steps

A step with a `tool` key is a [tool step](/reference/flow-tools/). Any other step is a request step. If a step fails, the remaining steps are skipped.

| Key | |
| --- | --- |
| `request` | Request file, relative to the project |
| `with` | Variables for this step only; highest precedence |
| `capture` | `name: expression`, saved to the critter's context |
| `expect` | Expected status; replaces the request's status checks |
| `assert` | Extra checks |
| `watch` | Expressions compared between scheduled runs |

## triggers

| Key | |
| --- | --- |
| `type` | `schedule` |
| `every` | `30s` minimum; `s`, `m`, `h`; a bare number is seconds |
| `env` | Environment; the active one when left out |

## alert

| `on` | |
| --- | --- |
| `failure` | When it starts failing, and when it recovers (the default) |
| `change` | When a `watch` value changes |
| `findings` | When a security tool finds something the previous run didn't |

## Critter file

```yaml title="critters/<critter>/critter.yml"
name: Admin
species: armadillo
notes: Full access on staging
login: login
variables:
  - name: user
    value: admin@example.com
  - name: password
    secret: true
```

`species`: `roadrunner`, `tortoise`, `gecko`, `elf-owl`, `armadillo`, `scorpion`, `jackalope`, `tumbleweed`.
