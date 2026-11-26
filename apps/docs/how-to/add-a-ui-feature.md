# Add a UI feature

The front end (`apps/desktop/ui/src`) is React 19, TypeScript, Tailwind 4, shadcn/Radix
components and Zustand stores. Data flows one way:

```
component  →  action  →  backend  →  store  →  component
```

The folder rules (what may import what) are in the UI section of
[ARCHITECTURE.md](../../../ARCHITECTURE.md#ui-appsdesktopuisrc). In short:

| Put it in…    | If it is…                                                                      |
| ------------- | ------------------------------------------------------------------------------ |
| `domain/`     | A pure function: parsing, formatting, diffing. No React, no backend. Test it.  |
| `stores/`     | State, plus synchronous setters. Stores never import each other or actions.    |
| `actions/`    | An async workflow: call the backend, update one or more stores, report errors. |
| `components/` | A generic presentational piece that doesn't read app state.                    |
| `features/`   | A screen or part of one: reads stores, calls actions.                          |
| `hooks/`      | App-wide lifecycle (events, focus) and keyboard handling.                      |

Run the UI checks from the repository root:

```sh
npm run typecheck --prefix apps/desktop/ui && npm run lint --prefix apps/desktop/ui && npm run format:check --prefix apps/desktop/ui && npm test --prefix apps/desktop/ui
```

`npm run dev --prefix apps/desktop/ui` serves the UI in a plain browser at `localhost:5173`, which is
quick for layout work, but backend calls only work inside the Tauri app.

## Recipe: a new piece of state and a workflow

1. **Store.** Add fields and methods to the store that owns the area (or a new
   `stores/<area>Store.ts` built like `monitorStore.ts`). Name methods for
   what happened (`schedulesLoaded`, `alertMissed`); plain UI flags get setters
   (`setEnvDialogOpen`). Give it a `reset` if it holds per-project data, and
   call that in `actions/project.ts` beside the other stores' resets.
2. **Action.** In `actions/<area>.ts`, write the async function a component
   will call. Read other stores with `useXStore.getState()`, call
   `backend.*`, then call store methods with the result. Wrap user-initiated
   work in `reportErrors` / `succeeded`, and background hints in
   `warnQuietly` (`lib/errors.ts`).
3. **Component.** In `features/<area>/`, select only what you render
   (`useMonitorStore((s) => s.paused)`) and call the action from handlers.

If two stores need updating together, that's an action's job: stores importing
each other breaks the `import/no-cycle` lint.

## Recipe: a dialog

App-level dialogs are mounted once in `app/App.tsx` and opened through a flag
in `uiStore`:

1. Add `fooOpen: boolean` and `setFooOpen` to `stores/uiStore.ts`.
2. Write `features/<area>/FooDialog.tsx` using `components/ui/dialog.tsx`,
   reading `fooOpen` and calling `setFooOpen`. Include a `DialogTitle` (use
   `className="sr-only"` if it shouldn't show) for accessibility.
3. Mount `<FooDialog />` in `App.tsx` beside the others.
4. Open it from wherever: `useUiStore.getState().setFooOpen(true)`.

For a simple question, don't write a dialog: use the shared ones.

```ts
useUiStore.getState().askPrompt({ title: "New folder", initial: "", onSubmit: async (name) => … });
useUiStore.getState().askConfirm({ title: "Delete users/?", description: "", action: "Delete", destructive: true, onConfirm: … });
```

## Recipe: a settings tab

Settings is `features/settings/SettingsDialog.tsx`, a vertical Radix `Tabs`.

1. Write `features/settings/FooTab.tsx`. Match `ThemeTab` and `AboutTab`: an
   uppercase section heading, then a bordered card of rows with the label on
   the left and the control on the right.
2. In `SettingsDialog.tsx`, add a `<NavItem value="foo"><Icon /> Foo</NavItem>`
   to the list and a matching `<Tabs.Content value="foo">`. To show it only
   sometimes, wrap both in the same condition.
3. App-wide preferences that should survive restarts go in
   `stores/settingsStore.ts`, which persists to localStorage. Per-project
   settings belong in the project (`.cactus/state.json`, through a backend
   command) instead.

## Recipe: a keyboard shortcut

Shortcuts are Ctrl/Cmd + a key, defined in one table so labels and handlers
can't drift.

1. Add an entry to `SHORTCUTS` in `app/shortcuts.ts`:

   ```ts
   history: { key: "h", description: "history" },
   ```

   The key is `KeyboardEvent.key`. Check it doesn't clash with an existing
   entry or a platform shortcut (Ctrl+Q, Ctrl+W).

2. Add its handler to `SHORTCUT_ACTIONS` in `hooks/useKeyboardShortcuts.ts`.
   That object is typed `Record<ShortcutId, …>`, so typecheck fails until you
   do.
3. Show it in tooltips with `withShortcut("History", "history")`, or the key
   caps with `shortcutKeys("history")` and `components/Kbd.tsx`.

Shortcuts other than the switcher (Ctrl+P) wait while a dialog is open.

## Recipe: reacting to backend events

Backend events (file changes, flow progress, scheduled runs, alerts) are
subscribed once in `hooks/useAppLifecycle.ts`. Add a `backendEvents.<name>.listen`
there and hand the payload to an action or store method. Adding a new event
type starts in Rust: see [Add a backend command or event](add-a-backend-command.md#adding-an-event).

## Style rules

- **Copy is terse.** Labels and titles only. No helper sentences under
  fields, no explanatory empty states, no caveat lists in dialogs, no
  taglines. Toasts and errors are short and don't add advice. If a line of
  explanation seems truly essential, it's one short sentence, at most.
- **Colours come from theme tokens** (`text-muted-foreground`, `bg-card`,
  `text-success`), never Tailwind's palette (`text-green-500`). Tokens are
  exposed as Tailwind colours in `index.css` (`--color-success: var(--success)`);
  add a line there for a new token. A test in `theme/themes.test.ts` scans every
  component for palette classes.
- **Icons** come from `lucide-react`.
- **Generated components** in `components/ui/` come from shadcn
  (`npx shadcn add <component>`); prefer composing them over editing them.
- **Critters** are signalled with their sprites or plain chips, not coloured
  lines or tinted backgrounds.
- **Nothing saves automatically.** Edits to requests live in `editorStore`
  buffers until Ctrl+S; follow the same model for anything that edits a file.

## Tests

Logic belongs in `domain/` precisely so it can be unit-tested with Vitest
(`foo.ts` + `foo.test.ts`). Stores can be tested directly too
(`stores/editorStore.test.ts`). There are no component render tests; keep
components thin enough not to need them, and check them in the running app.
