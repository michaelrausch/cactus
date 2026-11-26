/** Small count after a tab label; hidden when zero. */
export function CountBadge({ count }: { count: number }) {
  return count ? (
    <span className="ml-1 rounded-full bg-muted px-1.5 text-xs leading-4 text-muted-foreground tabular-nums">
      {count}
    </span>
  ) : null;
}

/** Marks a tab whose section is in use. */
export function ActiveDot({ visible }: { visible: boolean }) {
  return visible ? <span className="ml-1 size-1.5 rounded-full bg-brand" /> : null;
}
