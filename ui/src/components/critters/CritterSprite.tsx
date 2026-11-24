import { memo, type CSSProperties } from "react";
import { LoaderCircle } from "lucide-react";
import type { Species } from "@/api/client";
import { cn } from "@/lib/utils";
import { seedOf, spriteFrames, type CritterState } from "./species";

export type SpriteSize = 16 | 32 | 48;

interface CritterSpriteProps {
  species: Species;
  /** What it's doing: `work` while sending, `pass`/`fail` once (hop, shake). */
  state?: CritterState;
  /** Only these sizes keep the pixels square. */
  size?: SpriteSize;
  /** Read by screen readers, e.g. "Bob, roadrunner, sending". */
  label: string;
  /** Spreads blinks out so a list of critters doesn't blink in unison; defaults to the label. */
  seed?: string;
  /** Paused, e.g. its schedules are; shown as a "z" and no idle motion. */
  asleep?: boolean;
  className?: string;
}

/**
 * A critter as pixel art. Every frame of the state is drawn, stacked, and
 * CSS (`critters.css`) shows one at a time, so nothing re-renders while it
 * moves. Motion is never the only signal: states other than idle also show
 * a badge in the corner.
 */
export const CritterSprite = memo(function CritterSprite({
  species,
  state = "idle",
  size = 32,
  label,
  seed,
  asleep = false,
  className,
}: CritterSpriteProps) {
  const frames = spriteFrames(species, state);
  const style = { "--critter-size": `${size}px`, "--critter-seed": seedOf(seed ?? label) } as CSSProperties;
  return (
    <span
      role="img"
      aria-label={label}
      className={cn("critter", className)}
      data-state={state}
      data-asleep={asleep || undefined}
      style={style}
    >
      <svg
        className="critter-art"
        viewBox="0 0 16 16"
        width={size}
        height={size}
        shapeRendering="crispEdges"
        aria-hidden
      >
        {frames.map((frame) => (
          <g key={frame.key} className="critter-frame" data-frame={frame.index}>
            {frame.runs.map((run) => (
              <rect key={`${run.x},${run.y}`} x={run.x} y={run.y} width={run.width} height={1} fill={run.fill} />
            ))}
          </g>
        ))}
      </svg>
      <Badge state={state} asleep={asleep} />
    </span>
  );
});

const BADGE_GLYPHS: Record<Exclude<CritterState, "work"> | "asleep", string | null> = {
  idle: null,
  asleep: "z",
  pass: "✓",
  fail: "!",
};

function Badge({ state, asleep }: { state: CritterState; asleep: boolean }) {
  if (state === "work") {
    return (
      <span className="critter-badge" data-badge="work">
        <LoaderCircle className="animate-spin" />
      </span>
    );
  }
  const badge = state === "idle" && asleep ? "asleep" : state;
  const glyph = BADGE_GLYPHS[badge];
  if (!glyph) return null;
  return (
    <span className="critter-badge" data-badge={badge}>
      {glyph}
    </span>
  );
}
