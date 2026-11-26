---
title: Git
description: Stage, diff, commit and switch branches in the app.
---

<kbd>Ctrl</kbd>+<kbd>G</kbd> opens **Changes**. From there you can:

- view the diff of each changed file
- stage, unstage and discard, one file at a time or all at once
- commit (<kbd>Ctrl</kbd>+<kbd>Enter</kbd> in the message box)
- switch and create local branches

Push, pull and merge aren't built in.

The project can be the repository root or any folder inside it. Changes lists only the project's files, but a commit includes everything staged. If files outside the project are staged, Changes says so.

The commit author comes from your git config `user.name` and `user.email`, or from **Settings → Git**. Hooks don't run.

`.cactus/` contains its own `.gitignore`, so git ignores it.
