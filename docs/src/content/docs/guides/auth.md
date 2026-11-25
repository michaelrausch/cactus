---
title: Auth
description: Bearer, basic, API key, digest, OAuth2, and inheritance.
---

Set auth on a request's **Auth** tab (`http.auth`), or for a whole folder or collection (`request.auth`).

## Inherit

```yaml
auth: inherit
```

Uses the nearest auth set above: the request's folder, then that folder's parents, then the collection. New requests start with `inherit`.

A request with no `auth` key sends no credentials.

## Bearer

```yaml
auth:
  type: bearer
  token: "{{token}}"
```

## Basic

```yaml
auth:
  type: basic
  username: "{{user}}"
  password: "{{password}}"
```

## API key

```yaml
auth:
  type: apikey
  key: X-API-Key
  value: "{{apiKey}}"
  placement: query      # optional; a header when left out
```

## Digest

```yaml
auth:
  type: digest
  username: "{{user}}"
  password: "{{password}}"
```

Sent once without credentials. If the server replies `401` with a Digest challenge, cactus sends it again with the answer.

## OAuth2

cactus gets the token before sending, caches it, and refreshes it when it expires. If the server rejects a cached token with `401`, cactus gets a new one and retries once.

### Client credentials

```yaml
auth:
  type: oauth2
  flow: client_credentials
  accessTokenUrl: https://auth.example.com/oauth/token
  credentials:
    clientId: "{{clientId}}"
    clientSecret: "{{clientSecret}}"
  scope: read:orders
```

### Password

```yaml
auth:
  type: oauth2
  flow: resource_owner_password_credentials
  accessTokenUrl: https://auth.example.com/oauth/token
  credentials:
    clientId: "{{clientId}}"
  resourceOwner:
    username: "{{user}}"
    password: "{{password}}"
```

### Authorization code

```yaml
auth:
  type: oauth2
  flow: authorization_code
  authorizationUrl: https://auth.example.com/authorize
  accessTokenUrl: https://auth.example.com/oauth/token
  callbackUrl: http://127.0.0.1:7777/callback
  credentials:
    clientId: "{{clientId}}"
  scope: openid profile
  pkce: {}              # S256 by default
```

Click **Get token** on the Auth tab to sign in. The callback URL must be a loopback `http://` address. See the [OAuth2 recipe](/recipes/oauth2/).

### Options

| Field | |
| --- | --- |
| `credentials.placement: body` | Send the client id and secret in the form body instead of a Basic header |
| `refreshTokenUrl` | Where refresh requests go, if not the token URL |
| `additionalParameters` | Extra params for `authorizationRequest`, `accessTokenRequest`, `refreshTokenRequest`: `name`, `value`, optional `placement` (`header`, `query`, `body`) |
| `tokenConfig.id` | Requests with the same id share a token |
| `tokenConfig.placement` | `{header: Token}` or `{query: access_token}` |
| `tokenConfig.source: id_token` | Send the ID token instead of the access token |
| `settings.autoFetchToken: false` | Don't fetch a token while sending |
| `settings.autoRefreshToken: false` | Don't refresh an expired token while sending |

Tokens are stored per critter, or in `.cactus/tokens.json` when you aren't sending as one.

## Not sent

AWS Signature, NTLM, OAuth 1 and the OAuth2 implicit grant are kept in the file as written, but the request goes out without them.
