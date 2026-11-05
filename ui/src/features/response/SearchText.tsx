import { useEffect, useMemo, useRef, type ReactNode } from "react";

interface SearchTextProps {
  text: string;
  /** Length of each match, starting at every offset in `matches`. */
  queryLength: number;
  matches: number[];
  current: number;
}

function highlight(text: string, queryLength: number, matches: number[], current: number): ReactNode[] {
  const parts: ReactNode[] = [];
  let last = 0;
  matches.forEach((offset, index) => {
    parts.push(text.slice(last, offset));
    parts.push(
      <mark key={offset} data-current={index === current ? "" : undefined}>
        {text.slice(offset, offset + queryLength)}
      </mark>,
    );
    last = offset + queryLength;
  });
  parts.push(text.slice(last));
  return parts;
}

/** `<pre>` with highlighted matches; keeps the current match in view. */
export function SearchText({ text, queryLength, matches, current }: SearchTextProps) {
  const ref = useRef<HTMLPreElement>(null);
  const parts = useMemo(
    () => (matches.length ? highlight(text, queryLength, matches, current) : [text]),
    [text, queryLength, matches, current],
  );

  // Only when the current match moves, not on every render.
  useEffect(() => {
    if (matches[current] === undefined) return;
    ref.current?.querySelector("mark[data-current]")?.scrollIntoView({ block: "center" });
  }, [matches, current]);

  return (
    <pre ref={ref} className="font-mono text-xs leading-relaxed break-all whitespace-pre-wrap select-text">
      {parts}
    </pre>
  );
}
