import { useState } from "react";
import { answerFlowAsk, dismissFlowMessage } from "@/actions/flows";
import type { FlowAsked, FlowDisplayed } from "@/api/client";
import { Field } from "@/components/Field";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useCritterStore } from "@/stores/critterStore";
import { useFlowStore } from "@/stores/flowStore";

/** The key a choice is answered under, with its index. Mirrors `tools::CHOICE` in Rust. */
const CHOICE = "choice";

/** "Bob › Checkout", naming the flow that asks or shows something. */
function useAsker(from: { critter: string; flow: string }): string {
  const critter = useCritterStore((s) => s.critters.find((c) => c.id === from.critter));
  const flow = critter?.flows.find((f) => f.id === from.flow);
  return `${critter?.name ?? from.critter} › ${flow?.name ?? from.flow}`;
}

/** A display step's message; the run has carried on without it. */
function MessageBody({ message }: { message: FlowDisplayed }) {
  const asker = useAsker(message);
  return (
    <>
      <DialogHeader>
        <DialogTitle>{message.title}</DialogTitle>
        <DialogDescription>{asker}</DialogDescription>
      </DialogHeader>
      {message.text && <p className="text-sm break-words whitespace-pre-wrap">{message.text}</p>}
      <DialogFooter>
        <Button autoFocus onClick={dismissFlowMessage}>
          OK
        </Button>
      </DialogFooter>
    </>
  );
}

/** Mounted per question, so its fields start from that question's defaults. */
function AskForm({ ask }: { ask: FlowAsked }) {
  const asker = useAsker(ask);
  const question = ask.ask;
  const fields = question.kind === "fields" ? question.fields : [];
  const [values, setValues] = useState<Record<string, string>>(() =>
    Object.fromEntries(fields.map((field) => [field.name?.trim() ?? "", field.default ?? ""])),
  );
  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        void answerFlowAsk(values);
      }}
    >
      <DialogHeader>
        <DialogTitle>{question.title}</DialogTitle>
        <DialogDescription>{asker}</DialogDescription>
      </DialogHeader>
      {question.kind === "continue" && <p className="text-sm break-words whitespace-pre-wrap">{question.text}</p>}
      {question.kind === "choose" && (
        <ul aria-label={question.title} className="-mx-1 flex max-h-72 flex-col overflow-x-hidden overflow-y-auto">
          {question.choices.map((choice, index) => {
            // The values choosing it sets, beside its label.
            const detail = Object.values(choice.values)
              .filter((value) => value && value !== choice.label)
              .join(" · ");
            return (
              // Labels can repeat; a choice is its place in the list.
              // oxlint-disable-next-line react/no-array-index-key
              <li key={index}>
                <button
                  type="button"
                  autoFocus={index === 0}
                  title={Object.entries(choice.values)
                    .map(([name, value]) => `${name} = ${value}`)
                    .join("\n")}
                  onClick={() => void answerFlowAsk({ [CHOICE]: String(index) })}
                  className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm focus-ring hover:bg-accent/50"
                >
                  <span className="min-w-0 flex-1 truncate">{choice.label}</span>
                  {detail && (
                    <span className="max-w-[40%] shrink-0 truncate font-mono text-xs text-muted-foreground">
                      {detail}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
      {fields.map((field, index) => {
        const name = field.name?.trim() ?? "";
        return (
          <Field key={name} label={field.label?.trim() || name}>
            <Input
              // The first field takes focus, so typing answers straight away.
              autoFocus={index === 0}
              type={field.secret ? "password" : "text"}
              value={values[name] ?? ""}
              onChange={(event) => setValues({ ...values, [name]: event.target.value })}
            />
          </Field>
        );
      })}
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={() => void answerFlowAsk(null)}>
          Stop
        </Button>
        {question.kind !== "choose" && (
          <Button type="submit" autoFocus={fields.length === 0}>
            Continue
          </Button>
        )}
      </DialogFooter>
    </form>
  );
}

/**
 * A running flow's prompt or pause, which the run waits for (closing it
 * stops the run), or else its latest message, which it doesn't.
 */
export function FlowAskDialog() {
  const ask = useFlowStore((s) => s.ask);
  const message = useFlowStore((s) => s.message);
  if (ask) {
    return (
      <Dialog open onOpenChange={(open) => !open && void answerFlowAsk(null)}>
        {/* One column that may shrink: long choices truncate instead of widening the form past the dialog. */}
        <DialogContent className="grid-cols-1 sm:max-w-md">
          <AskForm key={ask.id} ask={ask} />
        </DialogContent>
      </Dialog>
    );
  }
  return (
    <Dialog open={!!message} onOpenChange={(open) => !open && dismissFlowMessage()}>
      <DialogContent className="grid-cols-1 sm:max-w-md">{message && <MessageBody message={message} />}</DialogContent>
    </Dialog>
  );
}
