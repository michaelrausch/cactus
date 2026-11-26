/**
 * The site's Worker: static files from `dist/`, plus Ko-fi's webhook, which
 * emails a cactus pro license key for each donation, whatever the amount.
 *
 * Secrets (`wrangler secret put …`): KOFI_VERIFICATION_TOKEN (Ko-fi →
 * Settings → API), LICENSE_SIGNING_KEY (the file `license-gen keypair`
 * wrote) and RESEND_API_KEY.
 */
import { issue } from "./license.ts";

interface Env {
  ASSETS: { fetch(request: Request): Promise<Response> };
  KOFI_VERIFICATION_TOKEN: string;
  LICENSE_SIGNING_KEY: string;
  RESEND_API_KEY: string;
  /** Who license emails are from, e.g. `cactus <licenses@usecactus.dev>`. */
  LICENSE_FROM: string;
}

/** The fields of Ko-fi's webhook this uses: https://ko-fi.com/manage/webhooks */
export interface KofiPayment {
  verification_token: string;
  kofi_transaction_id: string;
  /** Tip, Subscription, Commission or Shop Order: each gets a key. */
  type: string;
  from_name: string | null;
  email: string | null;
  amount: string;
  currency: string;
  is_subscription_payment: boolean;
  is_first_subscription_payment: boolean;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname === "/api/kofi" && request.method === "POST") return kofi(request, env);
    return env.ASSETS.fetch(request);
  },
};

async function kofi(request: Request, env: Env): Promise<Response> {
  const form = await request.formData();
  let payment: KofiPayment;
  try {
    payment = JSON.parse(String(form.get("data")));
  } catch {
    return new Response("Bad request", { status: 400 });
  }
  if (!sameText(payment.verification_token ?? "", env.KOFI_VERIFICATION_TOKEN)) {
    return new Response("Forbidden", { status: 403 });
  }

  const skip = skipReason(payment);
  if (skip) {
    console.log(`Ko-fi ${payment.kofi_transaction_id}: no key, ${skip}`);
    return new Response("OK");
  }

  // Signatures are deterministic, so if Ko-fi sends a payment again the
  // same key goes out again.
  const key = await issue(
    { id: payment.kofi_transaction_id, name: payment.from_name ?? "", email: payment.email ?? "" },
    env.LICENSE_SIGNING_KEY,
  );
  const sent = await sendKey(env, payment.email ?? "", key);
  if (!sent.ok) {
    console.error(`Ko-fi ${payment.kofi_transaction_id}: email failed: ${sent.status} ${await sent.text()}`);
    // Ko-fi retries a failed delivery.
    return new Response("Email failed", { status: 502 });
  }
  console.log(`Ko-fi ${payment.kofi_transaction_id}: key sent`);
  return new Response("OK");
}

/** Why `payment` gets no key, or null if it gets one. */
export function skipReason(payment: KofiPayment): string | null {
  if (payment.is_subscription_payment && !payment.is_first_subscription_payment) {
    return "a repeat subscription payment";
  }
  if (!payment.email) return "no email address";
  return null;
}

function sendKey(env: Env, to: string, key: string): Promise<Response> {
  const text = [
    "Thanks for supporting cactus.",
    "",
    "Your cactus pro license key:",
    "",
    key,
    "",
    "Paste it into cactus pro when it asks. Download cactus pro at https://usecactus.dev/pro",
  ].join("\n");
  return fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: { Authorization: `Bearer ${env.RESEND_API_KEY}`, "Content-Type": "application/json" },
    body: JSON.stringify({ from: env.LICENSE_FROM, to: [to], subject: "Your cactus pro license key", text }),
  });
}

/** Compare without stopping at the first difference, so timing doesn't give the token away. */
function sameText(a: string, b: string): boolean {
  const [x, y] = [new TextEncoder().encode(a), new TextEncoder().encode(b)];
  let diff = x.length ^ y.length;
  for (let i = 0; i < Math.max(x.length, y.length); i++) diff |= (x[i] ?? 0) ^ (y[i] ?? 0);
  return diff === 0 && b.length > 0;
}
