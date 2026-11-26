import { Hourglass } from "lucide-react";
import { bind, Fields, NumberInput } from "./fields";
import { defineTool } from "./types";

/** Mirrors `wait::MAX_SECONDS` in Rust. */
const MAX_SECONDS = 600;

export const wait = defineTool<"wait">({
  label: "Wait",
  icon: Hourglass,
  create: () => ({ tool: "wait", seconds: 5 }),
  summary: (step) => `${step.seconds ?? 0}s`,
  dataflow: (step) => ({
    problems: (step.seconds ?? 0) > MAX_SECONDS ? [`Wait at most ${MAX_SECONDS} seconds`] : [],
  }),
  Editor: (props) => (
    <Fields>
      <NumberInput label="Seconds" max={MAX_SECONDS} {...bind(props, "seconds")} />
    </Fields>
  ),
});
