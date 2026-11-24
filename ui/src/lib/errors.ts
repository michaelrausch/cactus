import { toast } from "sonner";

/** Backend errors arrive as strings; anything else is stringified. */
export const errorMessage = (error: unknown): string => (error instanceof Error ? error.message : String(error));

/**
 * Run `work`, showing a toast instead of letting a failure go unhandled.
 * `context` prefixes the message, e.g. `"Save failed"`.
 */
export async function reportErrors<T>(work: () => Promise<T>, context?: string): Promise<T | undefined> {
  try {
    return await work();
  } catch (error) {
    toast.error(context ? `${context}: ${errorMessage(error)}` : errorMessage(error));
    return undefined;
  }
}

/**
 * Run background `work` whose failure only costs a hint (highlighting, a diff
 * preview): log it instead of interrupting the user. Resolves to `undefined`
 * on failure. User-initiated work goes through {@link reportErrors} instead.
 */
export async function warnQuietly<T>(work: () => Promise<T>, context: string): Promise<T | undefined> {
  try {
    return await work();
  } catch (error) {
    console.warn(`${context}: ${errorMessage(error)}`);
    return undefined;
  }
}
