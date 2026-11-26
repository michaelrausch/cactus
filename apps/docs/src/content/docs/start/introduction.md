---
title: What cactus is
description: The project format and the parts of cactus.
---

cactus is an API client. A project is a folder of YAML files, one per request, in the [OpenCollection](https://schema.opencollection.com/) v1.0.0 format.

```
my-api/
├── opencollection.yml        # collection: name, shared headers and variables
├── environments/
│   ├── local.yml
│   └── staging.yml
├── auth/
│   ├── folder.yml
│   └── login.yml             # a request
├── users/
│   ├── list-users.yml
│   └── create-user.yml
└── critters/
    └── admin/
        ├── critter.yml       # a persona
        └── flows/
            └── smoke.yml     # one of its flows
```

When cactus saves a file it changes only the edited lines. Comments, blank lines and keys it doesn't know are kept.

## Ways to run a project

| | |
| --- | --- |
| Desktop app | macOS and Windows. Edit, send, build flows, commit. |
| `cactus-cli` | Run requests and flows headless, e.g. in CI. |
| MCP server | Lets an AI agent list, send and run what's in the project. |

All three read the same files.

## Terms

**Request**
: One `.yml` file. See [Requests](/guides/requests/).

**Environment**
: A named set of variables, e.g. `local` or `staging`. See [Variables](/guides/variables/).

**Check**
: A test on a response, e.g. `res.status eq 200`.

**Capture**
: Saves part of a response, e.g. `res.body.token`, as a variable. See [Checks and captures](/guides/checks-and-captures/).

**Critter**
: A persona with its own variables, secrets, cookies and captured values. See [Critters](/guides/critters/).

**Flow**
: Steps a critter runs in order: requests and tools such as prompt, pick, poll and Slack. See [Flows](/guides/flows/).

## Not supported

- Scripts. `runtime.scripts` are kept in the file but not run, and the response shows a warning.
- AWS Signature, NTLM, OAuth 1 and the OAuth2 implicit grant. They're kept in the file but not sent.
