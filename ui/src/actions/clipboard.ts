import { toast } from "sonner";
import { succeeded } from "@/lib/errors";

/** Copy `text`, confirming with `done`. */
export async function copyText(text: string, done: string): Promise<void> {
  if (await succeeded(() => navigator.clipboard.writeText(text), "Couldn't copy to the clipboard")) {
    toast.success(done);
  }
}
