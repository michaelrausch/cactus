import { createContext } from "react";

/**
 * The variable names in scope for the template fields below, which colour
 * and complete them; null while unknown, when nothing is flagged as unresolved.
 * The editor provides it, so the fields themselves read no app state.
 */
export const KnownNamesContext = createContext<readonly string[] | null>(null);

/**
 * Where each variable in scope gets its value, by name, e.g. `"staging"` or
 * `"Bob's context · auth/login.yml, 2m ago"`: shown with completions and as
 * the field's tooltip. Provided by the editor alongside {@link KnownNamesContext}.
 */
export const VariableDetailsContext = createContext<Readonly<Record<string, string>>>({});
