import { useRef } from "react";
import { editFlowText } from "@/actions/flows";
import { useFlowStore } from "@/stores/flowStore";

/** Matches `leading-5`: each line of the editor is 20px tall. */
const LINE_HEIGHT_PX = 20;
/** Matches `py-2` on the text and gutter. */
const TOP_PADDING_PX = 8;

/**
 * The flow file as text, saved exactly as typed. It is checked once typing
 * pauses: a line that doesn't parse is marked and explained.
 */
export function FlowYamlEditor() {
  const text = useFlowStore((s) => s.text);
  const problem = useFlowStore((s) => s.parseProblem);
  const gutter = useRef<HTMLDivElement>(null);
  const marker = useRef<HTMLDivElement>(null);
  const lineCount = text.split("\n").length;

  const syncScroll = (top: number) => {
    if (gutter.current) gutter.current.scrollTop = top;
    if (marker.current) marker.current.style.transform = `translateY(${-top}px)`;
  };

  return (
    <div className="flex h-full flex-col">
      <div className="relative flex min-h-0 flex-1 overflow-hidden font-mono text-xs leading-5">
        <div
          ref={gutter}
          aria-hidden
          className="w-10 shrink-0 overflow-hidden border-r py-2 pr-2 text-right text-muted-foreground/70 select-none"
        >
          {Array.from({ length: lineCount }, (_, i) => (
            <div key={i} className={problem?.line === i + 1 ? "text-danger" : undefined}>
              {i + 1}
            </div>
          ))}
        </div>
        <div className="relative min-w-0 flex-1">
          {problem && problem.line !== null && (
            <div ref={marker} aria-hidden className="pointer-events-none absolute inset-x-0">
              <div
                className="absolute inset-x-0 border-b-2 border-dotted border-danger bg-danger/10"
                style={{ top: TOP_PADDING_PX + (problem.line - 1) * LINE_HEIGHT_PX, height: LINE_HEIGHT_PX }}
              />
            </div>
          )}
          <textarea
            value={text}
            spellCheck={false}
            aria-label="Flow YAML"
            aria-invalid={problem !== null}
            onChange={(event) => editFlowText(event.target.value)}
            onScroll={(event) => syncScroll(event.currentTarget.scrollTop)}
            className="relative h-full w-full resize-none bg-transparent px-3 py-2 whitespace-pre outline-none"
          />
        </div>
      </div>
      {problem && (
        <p role="alert" className="border-t bg-danger/10 px-4 py-1.5 text-xs text-danger">
          {problem.line !== null && `Line ${problem.line}: `}
          {problem.message}
        </p>
      )}
    </div>
  );
}
