---
title: Sign in with OAuth2
description: OAuth2 set once on the collection, with browser sign-in and automatic refresh.
---

## Collection auth

```yaml title="opencollection.yml"
request:
  auth:
    type: oauth2
    flow: authorization_code
    authorizationUrl: https://{{authDomain}}/authorize
    accessTokenUrl: https://{{authDomain}}/oauth/token
    callbackUrl: http://127.0.0.1:7777/callback
    credentials:
      clientId: "{{clientId}}"
    scope: openid profile email offline_access
    pkce: {}
    additionalParameters:
      authorizationRequest:
        - { name: audience, value: "{{audience}}" }
      accessTokenRequest:
        - { name: audience, value: "{{audience}}" }
```

```yaml title="environments/staging.yml"
name: staging
variables:
  - { name: authDomain, value: staging-acme.eu.auth0.com }
  - { name: clientId, value: 3kd9Xa... }
  - { name: audience, value: https://api.staging.acme.dev }
```

Requests use it with `auth: inherit`.

1. Register `http://127.0.0.1:7777/callback` with the provider.
2. On any request's **Auth** tab, click **Get token** and sign in in the browser.
3. Sends now use the cached token and refresh it when it expires. `offline_access` (or your provider's equivalent) is needed to get a refresh token.

Tokens are cached per critter, so each critter can be signed in as a different user.

## Client credentials

No browser needed, so this works in CI:

```yaml
auth:
  type: oauth2
  flow: client_credentials
  accessTokenUrl: https://{{authDomain}}/oauth/token
  credentials:
    clientId: "{{clientId}}"
    clientSecret: "{{clientSecret}}"
    placement: body
```

## Troubleshooting

| Problem | Fix |
| --- | --- |
| Get token fails immediately | The callback port is in use. Change it here and with the provider. |
| Redirect URI mismatch | It must match exactly: `127.0.0.1` vs `localhost`, trailing slash. |
| No token sent | Check that the request has `auth: inherit` and no folder above sets other auth. |
| API wants `Token`, not `Bearer` | `tokenConfig: { placement: { header: Token } }` |
