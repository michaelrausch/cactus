import { createContext } from "react";

/**
 * The variable names in scope for the template fields below, which colour
 * and complete them; null while unknown, when nothing is flagged as unresolved.
 * The editor provides it, so the fields themselves read no app state.
 */
export const KnownNamesContext = createContext<readonly string[] | null>(null);
