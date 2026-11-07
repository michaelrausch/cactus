import { useState, type KeyboardEvent } from "react";
import { CircleCheck, GitCommitHorizontal } from "lucide-react";
import type { CommitSummary, GitStatus } from "@/api/client";
import { commit } from "@/actions/git";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Textarea } from "@/components/ui/textarea";
import { stagedFiles, unstagedFiles } from "@/domain/git";
import { timeAgo } from "@/lib/format";
import { useGitStore } from "@/stores/gitStore";
import { useUiStore } from "@/stores/uiStore";
import { BranchMenu } from "./BranchMenu";
import { ChangeList } from "./ChangeList";
import { DiffView } from "./DiffView";

/** Past this, a summary line gets cut off in `git log --oneline` and most UIs. */
const SUMMARY_LIMIT = 72;

const plural = (count: number, noun: string): string => `${count} ${noun}${count === 1 ? "" : "s"}`;

function LastCommit({ commit: last }: { commit: CommitSummary | null }) {
  if (!last) return <p className="truncate text-xs text-muted-foreground">No commits yet</p>;
  return (
    <p className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground" title={`${last.id} ${last.summary}`}>
      <GitCommitHorizontal className="size-3.5 shrink-0" />
      <span className="min-w-0 truncate">{last.summary}</span>
      <span className="shrink-0">· {timeAgo(last.at)} ago</span>
    </p>
  );
}

function CommitForm({ status }: { status: GitStatus }) {
  const [message, setMessage] = useState("");
  const [committing, setCommitting] = useState(false);
  const stagedCount = stagedFiles(status).length;
  const canCommit = stagedCount > 0 && !status.busy && message.trim() !== "" && !committing;
  const summaryLength = message.split("\n")[0]?.trim().length ?? 0;

  const submit = async () => {
    if (!canCommit) return;
    setCommitting(true);
    if (await commit(message)) setMessage("");
    setCommitting(false);
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      void submit();
    }
  };

  return (
    <form
      className="flex flex-col gap-2 border-t p-2"
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      {status.busy && <p className="text-xs text-warning">Finish the {status.busy} in progress to commit.</p>}
      {status.stagedOutside > 0 && (
        <p className="text-xs text-muted-foreground">
          {plural(status.stagedOutside, "file")} staged outside this collection will be committed too.
        </p>
      )}
      <div className="relative">
        <Textarea
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={stagedCount > 0 ? "Commit message" : "Stage changes to commit them"}
          className="max-h-40 min-h-16 pb-5 text-sm"
        />
        {summaryLength > SUMMARY_LIMIT && (
          <span className="absolute right-2 bottom-1 text-[10px] text-warning" title="Keep the first line short">
            {summaryLength}/{SUMMARY_LIMIT}
          </span>
        )}
      </div>
      <Button type="submit" size="sm" disabled={!canCommit}>
        {stagedCount > 0 ? `Commit ${plural(stagedCount, "file")}` : "Nothing staged"}
        {canCommit && <span className="ml-1 text-[10px] opacity-60">Ctrl+Enter</span>}
      </Button>
    </form>
  );
}

function CleanState({ status }: { status: GitStatus }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 text-center">
      <CircleCheck className="size-8 text-success" />
      <p className="text-sm font-medium">Nothing to commit</p>
      <p className="max-w-64 text-xs text-muted-foreground">
        Every file in this collection matches {status.lastCommit ? `commit ${status.lastCommit.id}` : "the repository"}.
      </p>
    </div>
  );
}

/** Stage, review and commit the project's changes. */
export function GitDialog() {
  const status = useGitStore((s) => s.status);
  const open = useUiStore((s) => s.gitDialogOpen);
  const setOpen = useUiStore((s) => s.setGitDialogOpen);

  if (!status) return null;
  const clean = status.files.length === 0;
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="gap-0 p-0 sm:max-w-5xl">
        <DialogTitle className="sr-only">Changes</DialogTitle>
        <DialogDescription className="sr-only">Stage, review and commit changes to this collection.</DialogDescription>
        <div className="flex h-[560px]">
          <div className="flex w-80 shrink-0 flex-col border-r bg-sidebar/60">
            <div className="flex shrink-0 flex-col gap-1 border-b px-4 py-3">
              <div className="flex min-w-0">
                <BranchMenu status={status} />
              </div>
              <LastCommit commit={status.lastCommit} />
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto p-2">
              <ChangeList side="staged" files={stagedFiles(status)} />
              <ChangeList side="unstaged" files={unstagedFiles(status)} />
            </div>
            <CommitForm status={status} />
          </div>
          {clean ? <CleanState status={status} /> : <DiffView />}
        </div>
      </DialogContent>
    </Dialog>
  );
}
