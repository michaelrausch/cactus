import { useEffect, useReducer } from "react";
import { timeAgo } from "@/lib/format";

const SECOND = 1000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;

/** Milliseconds until `timeAgo` would show a different value for something `age` ms old. */
function untilNextChange(age: number): number {
  let unit = HOUR;
  if (age < MINUTE) unit = SECOND;
  else if (age < HOUR) unit = MINUTE;
  // A little past the boundary, so the next render lands on the new value.
  return unit - (age % unit) + 50;
}

/** A compact age (`5s`, `3m`, …) that re-renders itself whenever the shown value changes. */
export function TimeAgo({ at, suffix = "" }: { at: number; suffix?: string }) {
  const [, rerender] = useReducer((n: number) => n + 1, 0);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    const schedule = () => {
      timer = setTimeout(
        () => {
          rerender();
          schedule();
        },
        untilNextChange(Math.max(0, Date.now() - at)),
      );
    };
    schedule();
    return () => clearTimeout(timer);
  }, [at]);
  return (
    <>
      {timeAgo(at)}
      {suffix}
    </>
  );
}
