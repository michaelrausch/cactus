import { useId } from "react";
import { cn } from "@/lib/utils";

/** The cactus mark. Size it with `className`, e.g. `size-12`; defaults to `size-5`. */
export function Logo({ className }: { className?: string }) {
  // Unique per instance: the sidebar and welcome screen can both render one.
  const gradientId = useId();
  return (
    <svg
      viewBox="0 0 20 20"
      role="img"
      aria-label="cactus"
      className={cn(
        "size-5 shrink-0 drop-shadow-[0_0_6px_color-mix(in_oklch,var(--color-brand-2)_35%,transparent)]",
        className,
      )}
    >
      <defs>
        <linearGradient id={gradientId} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="var(--color-brand-1)" />
          <stop offset="0.5" stopColor="var(--color-brand-2)" />
          <stop offset="1" stopColor="var(--color-brand-3)" />
        </linearGradient>
      </defs>
      <g fill={`url(#${gradientId})`}>
        <rect x="8" y="3.5" width="4" height="14.5" rx="2" />
        <rect x="3" y="7" width="3" height="6.5" rx="1.5" />
        <rect x="3" y="11" width="6" height="2.5" rx="1.25" />
        <rect x="14" y="5" width="3" height="6.5" rx="1.5" />
        <rect x="11" y="9" width="6" height="2.5" rx="1.25" />
      </g>
      <circle cx="10" cy="3" r="1.8" fill="var(--color-flower)" />
    </svg>
  );
}
