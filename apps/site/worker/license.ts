/**
 * cactus pro license keys, as `core/src/license.rs` checks them:
 * `CACTUS-PRO-` and the base32 (RFC 4648, no padding) of the licensee as
 * JSON followed by an Ed25519 signature over that JSON.
 */

export interface License {
  /** Tells keys apart: the Ko-fi transaction it was issued for. */
  id: string;
  name: string;
  email: string;
}

const PREFIX = "CACTUS-PRO-";
const BASE32 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/** PKCS #8 wrapping for a raw 32-byte Ed25519 private key, which Web Crypto can't import raw. */
const PKCS8_ED25519_PREFIX = [0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20];

/** A key for `license`, signed with `signingKey`: the hex that `license-gen keypair` writes. */
export async function issue(license: License, signingKey: string): Promise<string> {
  const payload = new TextEncoder().encode(JSON.stringify({ id: license.id, name: license.name, email: license.email }));
  const key = await crypto.subtle.importKey(
    "pkcs8",
    new Uint8Array([...PKCS8_ED25519_PREFIX, ...hexBytes(signingKey)]),
    { name: "Ed25519" },
    false,
    ["sign"],
  );
  const signature = new Uint8Array(await crypto.subtle.sign({ name: "Ed25519" }, key, payload));
  const bytes = new Uint8Array(payload.length + signature.length);
  bytes.set(payload);
  bytes.set(signature, payload.length);
  return PREFIX + base32(bytes);
}

function hexBytes(hex: string): Uint8Array {
  const clean = hex.trim();
  if (!/^[0-9a-fA-F]{64}$/.test(clean)) throw new Error("the signing key must be 64 hex characters");
  return Uint8Array.from(clean.match(/../g) ?? [], (pair) => parseInt(pair, 16));
}

function base32(bytes: Uint8Array): string {
  let out = "";
  let buffer = 0;
  let bits = 0;
  for (const byte of bytes) {
    buffer = (buffer << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += BASE32[(buffer >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += BASE32[(buffer << (5 - bits)) & 31];
  return out;
}
