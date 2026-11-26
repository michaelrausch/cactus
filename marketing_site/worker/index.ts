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
  /** Where replies to license emails go: a mailbox someone reads. */
  LICENSE_REPLY_TO?: string;
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
  if (!env.KOFI_VERIFICATION_TOKEN || !env.LICENSE_SIGNING_KEY || !env.RESEND_API_KEY) {
    console.error("A secret isn't set: KOFI_VERIFICATION_TOKEN, LICENSE_SIGNING_KEY and RESEND_API_KEY are all needed");
    return new Response("Not set up", { status: 500 });
  }
  // Trimmed: a secret piped in with `echo` ends in a newline.
  if (!sameText((payment.verification_token ?? "").trim(), env.KOFI_VERIFICATION_TOKEN.trim())) {
    // Lengths only, never the tokens: enough to tell a different token from a cut-off one.
    const [got, want] = [(payment.verification_token ?? "").trim().length, env.KOFI_VERIFICATION_TOKEN.trim().length];
    console.log(`Ko-fi webhook with the wrong verification token (${got} characters, expected ${want})`);
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
  // An HTML part as well as the text, and somewhere real to reply to: plain
  // text that's mostly a random-looking key, from an address that can't be
  // replied to, reads as spam to filters.
  const html = `<!doctype html>
<html>
  <body style="margin:0;padding:24px;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#212121;line-height:1.5">
    <p>Thanks for supporting cactus.</p>
    <p>Your cactus pro license key:</p>
    <p style="padding:12px 14px;border:1px solid #e0e0e0;border-radius:8px;background:#f7f7f7;font-family:ui-monospace,Menlo,monospace;font-size:13px;word-break:break-all">${key}</p>
    <p>Paste it into cactus pro when it asks. You can download cactus pro at <a href="https://usecactus.dev/pro">usecactus.dev/pro</a>.</p>
    <p>If anything goes wrong, reply to this email.</p>
  </body>
</html>`;
  return fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: { Authorization: `Bearer ${env.RESEND_API_KEY}`, "Content-Type": "application/json" },
    body: JSON.stringify({
      from: env.LICENSE_FROM,
      to: [to],
      ...(env.LICENSE_REPLY_TO ? { reply_to: env.LICENSE_REPLY_TO } : {}),
      subject: "Your cactus pro license key",
      text: `${text}\n\nIf anything goes wrong, reply to this email.`,
      html,
    }),
  });
}

/** Compare without stopping at the first difference, so timing doesn't give the token away. */
function sameText(a: string, b: string): boolean {
  const [x, y] = [new TextEncoder().encode(a), new TextEncoder().encode(b)];
  let diff = x.length ^ y.length;
  for (let i = 0; i < Math.max(x.length, y.length); i++) diff |= (x[i] ?? 0) ^ (y[i] ?? 0);
  return diff === 0 && b.length > 0;
}
