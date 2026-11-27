---
title: Variables and environments
description: Where variables are set and which value wins.
---

`{{name}}` works in URLs, params, headers, bodies and auth fields.

## Where variables are set

| Where | File |
| --- | --- |
| Collection | `opencollection.yml` → `request.variables` |
| Environment | `environments/<name>.yml` → `variables` |
| Folder | `<folder>/folder.yml` → `request.variables` |
| Request | `runtime.variables` |
| Secrets | `.cactus/secrets.json`, set under **Variables** (<kbd>Ctrl</kbd>+<kbd>E</kbd>) |
| Captured | `.cactus/captured.json`, written by captures |

```yaml title="environments/staging.yml"
name: staging
variables:
  - name: baseUrl
    value: https://staging.example.com
  - name: apiKey
    secret: true          # value is kept in .cactus/, not in this file
```

Pick the active environment in the sidebar. In the CLI, use `-e <name>`.

## Precedence

Lowest to highest:

1. Collection
2. Environment
3. Folders, outermost first
4. Request
5. Secrets
6. Captured

When sending as a [critter](/guides/critters/), the project's captured values are left out and these are added on top:

7. Bot variables
8. Bot secrets
9. Bot context (its captured values)
10. Values set by earlier tool steps in the flow run
11. The flow step's `with`

The **Variables** dialog shows each variable's value and which layer it came from.

## Rules

- Values are filled in once. A value that contains `{{b}}` stays as the text `{{b}}`.
- Unknown names are sent as written and listed as unresolved.
- `{{ name }}` is the same as `{{name}}`.
