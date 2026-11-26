---
title: Critters
description: Personas with their own variables, secrets, cookies and tokens.
---

A critter is a persona, such as Admin, Viewer or Anonymous. Each one has its own:

- variables and secrets
- context: values its requests captured
- cookie jar and OAuth2 tokens
- login flow (optional)
- [flows](/guides/flows/)

<kbd>Ctrl</kbd>+<kbd>B</kbd> opens Critters.

```yaml title="critters/admin/critter.yml"
name: Admin
species: armadillo
notes: Full access on staging
login: login          # flow to run after a 401
variables:
  - name: user
    value: admin@example.com
  - name: password
    secret: true
```

The folder name (`admin`) is the critter's id. Ids use letters, digits, `-` and `_`.

Species: `roadrunner`, `tortoise`, `gecko`, `elf-owl`, `armadillo`, `scorpion`, `jackalope`, `tumbleweed`.

## Sending as a critter

Pick one in the sidebar's critter picker. While a critter is selected, requests:

- use its variables, secrets and context
- save captures to its context only
- use its cookie jar

```sh
cactus-cli send . users/delete-user.yml --as viewer
cactus-cli run . admin --as admin
```

## Re-login

If a request sent as a critter gets `401` and the critter has a `login` flow, cactus runs that flow and resends the request once. Requests inside the login flow don't trigger this.

```yaml title="critters/admin/flows/login.yml"
name: Login
steps:
  - request: auth/login.yml
```

## Context

The critter's page lists captured values, where each came from and when. They can be edited or cleared there.

Context, secrets, cookies and tokens are stored in `.cactus/critters/<id>/`. Duplicating a critter copies `critter.yml` and its flows, not those files.
