/**
 * Both editions' modules fit `Edition` and `EditionThemes`, so code
 * typechecked against Pro's (`@edition` in tsconfig.json) works with free's
 * too. Types only.
 */
import type * as Free from "./free";
import type * as FreeThemes from "./free/themes";
import type * as Pro from "./pro";
import type * as ProThemes from "./pro/themes";
import type { Edition, EditionThemes } from "./types";

type Fits<T extends Edition> = T;
type FitsThemes<T extends EditionThemes> = T;

export type FreeFits = [Fits<typeof Free>, FitsThemes<typeof FreeThemes>];
export type ProFits = [Fits<typeof Pro>, FitsThemes<typeof ProThemes>];
