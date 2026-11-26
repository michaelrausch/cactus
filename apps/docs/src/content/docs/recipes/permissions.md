---
title: Check every role's permissions
description: One critter per role, with different expected statuses for the same requests.
---

## Critters

```yaml title="critters/admin/critter.yml"
name: Admin
login: login
variables:
  - { name: user, value: admin@example.com }
  - { name: password, secret: true }
```

```yaml title="critters/member/critter.yml"
name: Member
login: login
variables:
  - { name: user, value: member@example.com }
  - { name: password, secret: true }
```

```yaml title="critters/anon/critter.yml"
name: Anonymous
```

Admin and Member each have a login flow that runs `auth/login.yml`, which captures `token`. Each critter keeps its own token.

## Flows

```yaml title="critters/admin/flows/permissions.yml"
name: Permissions
steps:
  - request: users/list.yml
    expect: 200
  - request: users/create.yml
    with: { email: perm-admin@example.com }
    expect: 201
    capture: { userId: res.body.id }
  - request: users/delete.yml
    expect: 204
  - request: billing/invoices.yml
    expect: 200
```

```yaml title="critters/member/flows/permissions.yml"
name: Permissions
steps:
  - request: users/list.yml
    expect: 200
    assert:
      - { expression: res.body.users, operator: length, value: "1" }
  - request: users/create.yml
    with: { email: perm-member@example.com }
    expect: 403
  - request: billing/invoices.yml
    expect: 403
```

```yaml title="critters/anon/flows/permissions.yml"
name: Permissions
steps:
  - request: users/list.yml
    expect: 401
  - request: billing/invoices.yml
    expect: 401
```

`expect` replaces the request's own status check. Anonymous has no `login` flow, so a `401` isn't retried.

## Run

```sh
cactus-cli flow . --all -e staging --junit permissions.xml
```

## Notes

- Give the flow the same name for every critter so the report lines up.
- For each expected `403`, make sure some other critter gets a `2xx` from the same request. Otherwise the endpoint might just be broken.
- Check response bodies as well as statuses. A `200` can still return another user's data.
