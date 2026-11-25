---
title: Moving from Postman
description: Import a collection and replace its scripts.
---

## Import

Export as **Collection v2.1**, then:

```sh
mkdir api-tests
cactus-cli import api-tests Payments.postman_collection.json
```

## Environments

Postman environments aren't imported. Create one file for each:

```yaml title="environments/staging.yml"
name: staging
variables:
  - { name: baseUrl, value: https://staging.payments.example.com }
  - { name: apiKey, secret: true }
```

## Scripts

Scripts aren't run. Common replacements:

| Postman | cactus |
| --- | --- |
| `pm.response.to.have.status(200)` | Check `res.status eq 200` |
| `pm.expect(json.items.length).to.be.above(0)` | Check `res.body.items isNotEmpty` |
| `pm.environment.set("token", json.access_token)` | Capture `res.body.access_token` → `token` |
| `pm.variables.set("id", uuid())` | Flow step `set` with `id: $uuid` |
| Retry loops | Flow step `poll` |
| Collection runner | A flow, or `cactus-cli run` |

Delete a script from the **Scripts** tab once checks and captures do its job.

## Clean up

- Set auth once on the collection or a folder, and change requests to `auth: inherit`.
- Give the login request `seq: 1` and have it capture the token.
- Replace per-user environments (`staging-admin`, `staging-viewer`) with one environment plus a [critter](/guides/critters/) for each user.
