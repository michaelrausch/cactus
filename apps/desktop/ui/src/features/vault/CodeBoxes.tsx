import { useRef, type ClipboardEvent, type KeyboardEvent } from "react";
import { Input } from "@/components/ui/input";

const LENGTH = 6;

/**
 * A 6-digit authenticator code as one box per digit. Typing moves to the next
 * box, Backspace on an empty box to the previous one, and a pasted code fills
 * them all. `onComplete` runs once every box has a digit.
 */
export function CodeBoxes({
  value,
  onChange,
  onComplete,
  disabled,
}: {
  value: string;
  onChange: (value: string) => void;
  onComplete: (code: string) => void;
  disabled?: boolean;
}) {
  const boxes = useRef<(HTMLInputElement | null)[]>([]);
  const digits = Array.from({ length: LENGTH }, (_, i) => value[i] ?? "");
  const focus = (index: number) => boxes.current[Math.max(0, Math.min(LENGTH - 1, index))]?.focus();

  const update = (next: string) => {
    const code = next.replace(/\D/g, "").slice(0, LENGTH);
    onChange(code);
    if (code.length === LENGTH) onComplete(code);
    return code;
  };

  const type = (index: number, typed: string) => {
    const digit = typed.replace(/\D/g, "").slice(-1);
    if (!digit) return;
    const code = update(value.slice(0, index) + digit + value.slice(index + 1));
    focus(Math.min(code.length, index + 1));
  };

  const onKeyDown = (index: number, event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Backspace") {
      event.preventDefault();
      if (digits[index]) {
        update(value.slice(0, index) + value.slice(index + 1));
      } else if (index > 0) {
        update(value.slice(0, index - 1) + value.slice(index));
        focus(index - 1);
      }
    } else if (event.key === "ArrowLeft") {
      focus(index - 1);
    } else if (event.key === "ArrowRight") {
      focus(index + 1);
    }
  };

  const onPaste = (event: ClipboardEvent<HTMLInputElement>) => {
    event.preventDefault();
    const code = update(event.clipboardData.getData("text"));
    focus(code.length);
  };

  return (
    <div className="flex justify-center gap-2" role="group" aria-label="Authenticator code">
      {digits.map((digit, index) => (
        <Input
          // The boxes never reorder.
          // oxlint-disable-next-line react/no-array-index-key
          key={index}
          ref={(element) => {
            boxes.current[index] = element;
          }}
          value={digit}
          disabled={disabled}
          // Focus lands on the first empty box.
          autoFocus={index === Math.min(value.length, LENGTH - 1)}
          inputMode="numeric"
          autoComplete={index === 0 ? "one-time-code" : "off"}
          aria-label={`Digit ${index + 1}`}
          className="size-11 p-0 text-center font-mono text-lg"
          onFocus={(event) => event.target.select()}
          onChange={(event) => type(index, event.target.value)}
          onKeyDown={(event) => onKeyDown(index, event)}
          onPaste={onPaste}
        />
      ))}
    </div>
  );
}
