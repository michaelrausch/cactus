---
title: Importing
description: Import Postman collections and OpenAPI / Swagger specs.
---

Supported formats:

- Postman collection v2.x (JSON)
- OpenAPI 3 and Swagger 2 (JSON or YAML)

**App**: **Import Postman / OpenAPI** at the top of the sidebar.

**CLI**:

```sh
cactus-cli import . payments.postman_collection.json
```

```
imported 42 requests from postman
added to opencollection.yml: baseUrl
```

Importing only adds files. Variables already in `opencollection.yml` keep their values.

## Postman

| Imported | |
| --- | --- |
| Folders and requests | Order kept |
| Method, URL, params, headers | |
| Bodies | Raw, form, multipart |
| Auth | Bearer, basic, API key |
| Descriptions | As request docs |
| Collection variables | Added to `opencollection.yml` |
| Scripts | Kept in the file, not run |

Environments aren't imported.

## OpenAPI / Swagger

| Imported | |
| --- | --- |
| Operations | One request each, named from `summary` or `operationId` |
| Tags | First tag becomes the folder |
| Server URL | `baseUrl` collection variable; URLs become `{{baseUrl}}/path` |
| Path params | `/pets/{id}` becomes `/pets/:id` with an empty value |
| JSON bodies | From the spec's example, or `{}` |
| Form bodies | Field names, empty values |
| Descriptions | As request docs |

Query and header parameters aren't imported.
