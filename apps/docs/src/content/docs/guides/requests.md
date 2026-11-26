---
title: Requests
description: The request file and every body type.
---

Every `.yml` file is a request, except `opencollection.yml`, `folder.yml`, and anything under `environments/` or `critters/`.

## Example

```yaml title="users/create-user.yml"
info:
  name: Create user
  type: http
  seq: 2                     # order in its folder
  tags: [admin]
http:
  method: POST
  url: "{{baseUrl}}/users"
  headers:
    - name: Idempotency-Key
      value: "{{requestId}}"
  body:
    type: json
    data: |-
      { "email": "{{email}}", "role": "member" }
  auth: inherit
runtime:
  assertions:
    - expression: res.status
      operator: eq
      value: "201"
  actions:
    - type: set-variable
      phase: after-response
      selector:
        expression: res.body.id
        method: jsonq
      variable:
        name: userId
        scope: runtime
settings:
  timeout: 10000             # ms; default 30000
docs: Creates a member and saves its id as {{userId}}.
```

Only `info` and `http` are required.

## Folders

A folder is a directory. An optional `folder.yml` sets its name, its order, and defaults for the requests inside it:

```yaml title="admin/folder.yml"
info:
  name: Admin
  type: folder
  seq: 3
request:
  headers:
    - name: X-Admin
      value: "true"
  auth:
    type: bearer
    token: "{{adminToken}}"
```

At each level, folders and requests are sorted together by `seq`. `cactus-cli run` sends them in that order.

## Params

`params` mirrors the URL's query string. Edit either one and cactus updates the other.

```yaml
http:
  url: "{{baseUrl}}/users/:id?expand=orders"
  params:
    - name: id
      value: "{{userId}}"
      type: path
    - name: expand
      value: orders
      type: query
```

`disabled: true` keeps a param or header in the file without sending it.

## Headers

Headers come from the collection, then each enclosing folder, then the request itself. A header set later replaces an earlier one with the same name.

## Bodies

| `type` | `data` |
| --- | --- |
| `json` | JSON text |
| `text`, `xml`, `sparql` | Text |
| `form-urlencoded` | `name`/`value` list |
| `multipart-form` | List of text or file parts |
| `file` | A project file sent as the body |

```yaml
body:
  type: form-urlencoded
  data:
    - name: grant_type
      value: password
```

```yaml
body:
  type: multipart-form
  data:
    - name: title
      type: text
      value: Report
    - name: file
      type: file
      value:
        - fixtures/report.pdf
```

```yaml
body:
  type: file
  data:
    - filePath: fixtures/avatar.png
      contentType: image/png
      selected: true
```

File paths are relative to the project and can't point outside it.

Variables are inserted as text. Quote a value that should be a string; leave quotes off one that should be a number: `"count": {{count}}`.

## Sending

| Keys | |
| --- | --- |
| <kbd>Ctrl</kbd>+<kbd>Enter</kbd> | Send, including unsaved edits |
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save |
| <kbd>Ctrl</kbd>+<kbd>P</kbd> | Jump to a request |
| <kbd>Ctrl</kbd>+<kbd>D</kbd> | Duplicate |

A variable with no value is sent as written (`{{name}}`) and listed as unresolved next to the response.

Past responses are kept per request in `.cactus/history/`.

## Outside edits

If a file changes on disk while you have unsaved edits to it, cactus asks whether to reload it or keep yours. A file that doesn't parse is never overwritten.
