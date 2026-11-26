// `npm test`: Node's runner, which runs TypeScript as is.
import assert from "node:assert/strict";
import { test } from "node:test";
import { skipReason, type KofiPayment } from "./index.ts";
import { issue } from "./license.ts";

test("keys match the app's (crates/core/src/license.rs, keys_match_the_websites)", async () => {
  const key = await issue({ id: "test", name: "Ada", email: "ada@example.com" }, "07".repeat(32));
  assert.equal(
    key,
    "CACTUS-PRO-PMRGSZBCHIRHIZLTOQRCYITOMFWWKIR2EJAWIYJCFQRGK3LBNFWCEORCMFSGCQDFPBQW24DMMUXGG33NEJ6UBVGZWFY4WS6L6HO6BCH3226F5RRE53IKVNVXIXDK7QC4ZSIY5UDQELRLYYTOYGTDX3F55Z76UKGDZKVEORVRJUHQ4BRCDT4TIIN2A4",
  );
});

test("a bad signing key is refused", async () => {
  await assert.rejects(issue({ id: "t", name: "", email: "" }, "not hex"));
});

const payment = (fields: Partial<KofiPayment>): KofiPayment => ({
  verification_token: "token",
  kofi_transaction_id: "txn",
  type: "Tip",
  from_name: "Ada",
  email: "ada@example.com",
  amount: "5.00",
  currency: "USD",
  is_subscription_payment: false,
  is_first_subscription_payment: false,
  ...fields,
});

test("every donation gets a key, whatever the amount", () => {
  assert.equal(skipReason(payment({})), null);
  assert.equal(skipReason(payment({ amount: "1.00" })), null);
  assert.equal(skipReason(payment({ is_subscription_payment: true, is_first_subscription_payment: true })), null);
});

test("repeat subscription payments and missing emails don't", () => {
  assert.match(skipReason(payment({ is_subscription_payment: true })) ?? "", /repeat/);
  assert.match(skipReason(payment({ email: null })) ?? "", /email/);
});
