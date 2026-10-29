import { makeRow, type Row } from "./rows";

interface UrlParts {
  base: string;
  query: string;
  hash: string;
}

/**
 * Split a URL into base, query and hash without decoding anything, so
 * `{{vars}}` and pre-encoded values round-trip exactly.
 */
function splitUrl(url: string): UrlParts {
  const hashAt = url.indexOf("#");
  const hash = hashAt >= 0 ? url.slice(hashAt) : "";
  const withoutHash = hashAt >= 0 ? url.slice(0, hashAt) : url;
  const queryAt = withoutHash.indexOf("?");
  if (queryAt < 0) return { base: withoutHash, query: "", hash };
  return { base: withoutHash.slice(0, queryAt), query: withoutHash.slice(queryAt + 1), hash };
}

function parsePair(pair: string): Row {
  const eq = pair.indexOf("=");
  return eq >= 0 ? makeRow(pair.slice(0, eq), pair.slice(eq + 1)) : makeRow(pair);
}

export const parseQuery = (url: string): Row[] => splitUrl(url).query.split("&").filter(Boolean).map(parsePair);

export function withQuery(url: string, rows: Row[]): string {
  const { base, hash } = splitUrl(url);
  const query = rows
    .filter((row) => row.key)
    .map((row) => (row.value ? `${row.key}=${row.value}` : row.key))
    .join("&");
  return `${base}${query ? `?${query}` : ""}${hash}`;
}
