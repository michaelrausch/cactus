---
title: Install
description: Install the desktop app and cactus-cli.
---

## Desktop app

Download from [usecactus.dev](https://usecactus.dev/#download):

- **macOS**: universal `.dmg`
- **Windows**: `-setup.exe` installer, or the portable `.exe`

Open a folder with <kbd>Ctrl</kbd>+<kbd>O</kbd>. An empty folder becomes a new project.

The app tells you when a new version is out.

## cactus-cli

Build from source with Rust (stable):

```sh
cargo install --git https://github.com/michaelrausch/cactus cactus-cli --locked
```

```sh
cactus-cli --help
```

## Demo project

`examples/demo` in the repository calls [httpbin.org](https://httpbin.org):

```sh
cactus-cli run examples/demo
cactus-cli flow examples/demo admin smoke
```
