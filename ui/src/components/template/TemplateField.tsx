import { useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { createPortal } from "react-dom";
import { firstOf, placeholderSuggester, type Suggester, type Suggestion } from "@/domain/suggestions";
import { useKnownVariables } from "@/hooks/useKnownVariables";
import { cn } from "@/lib/utils";
import { highlight, type Syntax } from "./highlight";

interface TemplateFieldProps {
  value: string;
  onChange: (value: string) => void;
  /** A growing textarea instead of a one-line input. */
  multiline?: boolean;
  /** `json` also colours JSON syntax around the placeholders. */
  syntax?: Syntax;
  /** Colour and complete `{{name}}` placeholders. */
  templates?: boolean;
  /** More completions, used when no placeholder is being typed. */
  suggest?: Suggester | undefined;
  placeholder?: string | undefined;
  /** The box: layout, border, background. */
  className?: string;
  /** Text metrics shared by the input and the colours behind it: padding, font, size, height. */
  fieldClassName?: string;
  "aria-label"?: string;
}

interface Menu {
  items: Suggestion[];
  active: number;
  /** Where the caret was when the items were made, to place the list. */
  cursor: number;
}

/** What the input and its mirrors must share for the text to line up. */
const TEXT = "m-0 border-0 p-0 font-[inherit] tracking-normal";
const WRAPPING = "whitespace-pre-wrap break-words [overflow-wrap:anywhere]";
const GAP = 4;

/**
 * A text input that colours `{{placeholders}}` (and optionally JSON) and
 * offers completions. The real input sits on top with transparent text, over
 * a copy of the text drawn in colour, so editing behaves exactly as normal.
 */
export function TemplateField({
  value,
  onChange,
  multiline = false,
  syntax = "template",
  templates = true,
  suggest,
  placeholder,
  className,
  fieldClassName,
  "aria-label": ariaLabel,
}: TemplateFieldProps) {
  const known = useKnownVariables();
  const knownSet = useMemo(() => (known ? new Set(known) : null), [known]);
  const suggester = useMemo(
    () => firstOf(templates ? placeholderSuggester(known ?? []) : undefined, suggest),
    [templates, known, suggest],
  );

  const inputRef = useRef<HTMLInputElement & HTMLTextAreaElement>(null);
  const overlayRef = useRef<HTMLSpanElement>(null);
  const caretMirrorRef = useRef<HTMLSpanElement>(null);
  const caretRef = useRef<HTMLSpanElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  /** Caret to restore once an accepted completion has come back as `value`. */
  const pendingCaret = useRef<number | null>(null);
  const [menu, setMenu] = useState<Menu | null>(null);

  const coloured = useMemo(
    () => (templates || syntax !== "template" ? highlight(value, syntax, knownSet) : value),
    [value, syntax, templates, knownSet],
  );

  /** A one-line input scrolls sideways; its coloured copy has to follow. */
  const syncScroll = () => {
    const input = inputRef.current;
    if (multiline || !input) return;
    const shift = `translateX(${-input.scrollLeft}px)`;
    if (overlayRef.current) overlayRef.current.style.transform = shift;
    if (caretMirrorRef.current) caretMirrorRef.current.style.transform = shift;
  };

  const refresh = (text: string, cursor: number | null) => {
    const items = cursor === null ? [] : suggester(text, cursor);
    setMenu(items.length > 0 && cursor !== null ? { items, active: 0, cursor } : null);
  };

  const accept = (item: Suggestion) => {
    const next = value.slice(0, item.from) + item.insert + value.slice(item.to);
    const caret = item.from + item.insert.length;
    pendingCaret.current = caret;
    onChange(next);
    // A completion like `res.body.` leads straight on to the next level.
    refresh(next, caret);
  };

  useLayoutEffect(() => {
    const input = inputRef.current;
    if (input && pendingCaret.current !== null && document.activeElement === input) {
      input.setSelectionRange(pendingCaret.current, pendingCaret.current);
    }
    pendingCaret.current = null;
    syncScroll();
  });

  // Place the list under the caret, flipping above it near the bottom of the window.
  useLayoutEffect(() => {
    const list = listRef.current;
    const caret = caretRef.current;
    if (!menu || !list || !caret) return;
    const rect = caret.getBoundingClientRect();
    const below = rect.bottom + GAP;
    const fitsBelow = below + list.offsetHeight <= window.innerHeight;
    list.style.top = `${fitsBelow ? below : Math.max(GAP, rect.top - GAP - list.offsetHeight)}px`;
    list.style.left = `${Math.max(GAP, Math.min(rect.left, window.innerWidth - list.offsetWidth - GAP))}px`;
    list.children[menu.active]?.scrollIntoView({ block: "nearest" });
  }, [menu]);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === " " && event.ctrlKey) {
      event.preventDefault();
      refresh(value, inputRef.current?.selectionStart ?? null);
      return;
    }
    if (!menu) return;
    const count = menu.items.length;
    const move = (by: number) => setMenu({ ...menu, active: (menu.active + by + count) % count });
    const item = menu.items[menu.active];
    switch (event.key) {
      case "ArrowDown":
        move(1);
        break;
      case "ArrowUp":
        move(-1);
        break;
      case "Enter":
      case "Tab":
        if (!item || event.ctrlKey || event.metaKey) return;
        accept(item);
        break;
      case "Escape":
        event.stopPropagation();
        setMenu(null);
        break;
      default:
        return;
    }
    event.preventDefault();
  };

  const shared = cn(TEXT, fieldClassName);
  const inputClass = cn(
    shared,
    "relative w-full min-w-0 bg-transparent text-transparent caret-foreground outline-none placeholder:text-muted-foreground selection:bg-info/30",
  );
  const inputProps = {
    ref: inputRef,
    value,
    placeholder,
    spellCheck: false,
    "aria-label": ariaLabel,
    "aria-autocomplete": "list" as const,
    "aria-expanded": menu !== null,
    onChange: (event: { target: HTMLInputElement | HTMLTextAreaElement }) => {
      onChange(event.target.value);
      refresh(event.target.value, event.target.selectionStart);
    },
    onSelect: () => {
      syncScroll();
      if (menu) refresh(value, inputRef.current?.selectionStart ?? null);
    },
    onScroll: syncScroll,
    onKeyDown,
    onBlur: () => setMenu(null),
  };

  // A trailing newline needs something after it to take up a line.
  const mirrorText = multiline && (value === "" || value.endsWith("\n")) ? " " : "";
  const caretMirror = menu && (
    <span
      aria-hidden
      className={cn(
        shared,
        "pointer-events-none invisible absolute inset-0 overflow-hidden",
        multiline ? WRAPPING : "flex items-center whitespace-pre",
      )}
    >
      <span ref={caretMirrorRef} className={multiline ? undefined : "inline-block"}>
        {value.slice(0, menu.cursor)}
        <span ref={caretRef}>{"\u200b"}</span>
      </span>
    </span>
  );

  return (
    <div className={cn("relative min-w-0", multiline && "grid", className)}>
      {multiline ? (
        <>
          <div aria-hidden className={cn(shared, WRAPPING, "pointer-events-none col-start-1 row-start-1")}>
            <span ref={overlayRef}>{coloured}</span>
            {mirrorText}
          </div>
          <textarea
            {...inputProps}
            rows={1}
            className={cn(inputClass, WRAPPING, "col-start-1 row-start-1 h-full resize-none overflow-hidden")}
          />
        </>
      ) : (
        <>
          <span
            aria-hidden
            className={cn(
              shared,
              "pointer-events-none absolute inset-0 flex items-center overflow-hidden whitespace-pre",
            )}
          >
            <span ref={overlayRef} className="inline-block">
              {coloured}
            </span>
          </span>
          <input {...inputProps} className={inputClass} />
        </>
      )}
      {caretMirror}
      {menu &&
        createPortal(
          <div
            ref={listRef}
            role="listbox"
            // Keep focus (and the caret) in the input while picking with the mouse.
            onMouseDown={(event) => event.preventDefault()}
            className="pointer-events-auto fixed z-[100] max-h-60 min-w-48 overflow-y-auto rounded-lg border bg-popover p-1 font-mono text-xs text-popover-foreground shadow-lg shadow-black/40"
          >
            {menu.items.map((item, index) => (
              <div
                key={`${item.label}:${item.insert}`}
                role="option"
                aria-selected={index === menu.active}
                onMouseEnter={() => setMenu({ ...menu, active: index })}
                onClick={() => accept(item)}
                className={cn(
                  "flex cursor-default items-center gap-4 rounded-md px-2 py-1",
                  index === menu.active && "bg-accent",
                )}
              >
                <span className="truncate">{item.label}</span>
                {item.detail && <span className="ml-auto shrink-0 text-muted-foreground">{item.detail}</span>}
              </div>
            ))}
          </div>,
          document.body,
        )}
    </div>
  );
}
