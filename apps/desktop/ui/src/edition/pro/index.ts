/** The Pro edition's tools, bundled into the Pro build only. Its themes are in `themes.ts` and `themes.css`. */
import { cert } from "@/features/critters/flows/tools/cert";
import { fuzz } from "@/features/critters/flows/tools/fuzz";
import { headers } from "@/features/critters/flows/tools/headers";
import { loadtest } from "@/features/critters/flows/tools/loadtest";
import { ratelimit } from "@/features/critters/flows/tools/ratelimit";
import { slack } from "@/features/critters/flows/tools/slack";
import { webhook } from "@/features/critters/flows/tools/webhook";
import type { Edition } from "../types";

export { PRO } from "./flag";
export const PRO_TOOLS: Edition["PRO_TOOLS"] = { webhook, slack, fuzz, headers, cert, ratelimit, loadtest };
