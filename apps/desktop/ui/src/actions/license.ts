import { backend } from "@/api/client";
import { errorMessage } from "@/lib/errors";
import { useEditionStore } from "@/stores/editionStore";

/** Ask the backend who this copy is licensed to. */
export async function loadEdition(): Promise<void> {
  const { licensee } = await backend.edition();
  useEditionStore.getState().editionLoaded(licensee);
}

/** Enter a Pro license key. Resolves to an error to show, or null when done. */
export async function activateLicense(key: string): Promise<string | null> {
  try {
    const { licensee } = await backend.activateLicense(key.trim());
    useEditionStore.getState().editionLoaded(licensee);
    return null;
  } catch (error) {
    return errorMessage(error);
  }
}
