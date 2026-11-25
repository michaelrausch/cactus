---
title: API tests in CI
description: Run flows on every pull request with JUnit results.
---

## Flows

```yaml title="critters/ci/critter.yml"
name: CI
login: login
variables:
  - name: user
    value: ci@example.com
```

```yaml title="critters/ci/flows/login.yml"
name: Login
steps:
  - tool: prompt
    vars:
      - name: password
        secret: true
  - request: auth/login.yml
```

```yaml title="critters/ci/flows/smoke.yml"
name: Smoke
steps:
  - tool: run
    flow: login
  - tool: set
    values:
      email: $randomEmail
  - request: users/create.yml
    expect: 201
    capture:
      userId: res.body.id
  - request: users/get.yml
    expect: 200
  - request: users/delete.yml
    expect: 204
```

```yaml title="environments/ci.yml"
name: ci
variables:
  - name: baseUrl
    value: http://localhost:8080
```

## GitHub Actions

```yaml title=".github/workflows/api.yml"
name: API tests
on: [pull_request]

jobs:
  api:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: docker compose up -d --wait
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install --git https://github.com/michaelrausch/cactus cactus-cli --locked
      - run: >
          cactus-cli flow api-tests --all -e ci
          --var "password=${{ secrets.CI_USER_PASSWORD }}"
          --junit api-results.xml
      - if: always()
        uses: mikepenz/action-junit-report@v4
        with:
          report_paths: api-results.xml
```

`api-tests` is the project folder. The JUnit report has one suite per critter and one test case per step.

## Variations

Wait for the server to come up. Connection errors count as "not done yet":

```yaml
- tool: poll
  request: health.yml
  until:
    - { expression: res.status, operator: eq, value: "200" }
  timeout: 90
```

Send every request without flows:

```sh
cactus-cli run api-tests -e ci
```

One job per critter:

```yaml
strategy:
  matrix:
    critter: [admin, member, viewer]
steps:
  - run: cactus-cli flow api-tests ${{ matrix.critter }} smoke -e ci
```

## Exit codes

| | |
| --- | --- |
| `0` | Passed |
| `1` | A step or check failed |
| `2` | Couldn't run, e.g. bad path or unknown environment |
