import { toast } from "sonner";
import { backend } from "@/api/client";
import { reportErrors, warnQuietly } from "@/lib/errors";

let checked = false;

/**
 * Ask the website for a newer version, and offer it until dismissed. Offline is fine: it just says nothing.
 * Once per launch, however often it's called (StrictMode runs start-up effects twice).
 */
export async function checkForUpdate(): Promise<void> {
  if (checked) return;
  checked = true;
  const latest = await warnQuietly(() => backend.checkForUpdate(), "Couldn't check for updates");
  if (!latest) return;
  toast("Update available", {
    id: "update-available",
    description: `Version ${latest}`,
    duration: Infinity,
    closeButton: true,
    action: { label: "Download", onClick: () => void openWebsite() },
  });
}

function openWebsite(): Promise<unknown> {
  return reportErrors(() => backend.openWebsite(), "Couldn't open the website");
}
