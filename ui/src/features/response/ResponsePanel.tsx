import { useEffect, useState } from "react";
import { Loader2, RotateCw } from "lucide-react";
import { sendCurrentScenario } from "@/actions/request";
import { shortcutKeys } from "@/app/shortcuts";
import { CountBadge } from "@/components/Badges";
import { MethodBadge } from "@/components/MethodBadge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editorStore";
import { useResponseStore } from "@/stores/responseStore";
import type { HttpResponse } from "@/api/client";
import { BodyViewer } from "./BodyViewer";
import { ChecksList } from "./ChecksList";
import { HeadersTable } from "./HeadersTable";
import { HistoryList } from "./HistoryList";
import { ResponseSummary } from "./ResponseSummary";

/** Seconds since the calling component mounted, ticking. */
function useElapsedSeconds(): number {
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    const started = Date.now();
    const timer = setInterval(() => setElapsed((Date.now() - started) / 1000), 100);
    return () => clearInterval(timer);
  }, []);
  return elapsed;
}

/** While a send is in flight: a thin running bar and the time so far. */
function SendingBar() {
  const elapsed = useElapsedSeconds();
  return (
    <div role="status" className="flex flex-col gap-1.5">
      <div className="h-0.5 overflow-hidden rounded-full bg-muted">
        <div className="h-full w-1/3 animate-pulse rounded-full bg-brand" />
      </div>
      <p className="flex items-center gap-1.5 text-xs text-muted-foreground tabular-nums">
        <Loader2 className="size-3 animate-spin" /> Sending… {elapsed.toFixed(1)} s
      </p>
    </div>
  );
}

/** The send failed before any response came back: what was sent, why, and a retry. */
function RequestFailed({ error }: { error: string }) {
  const method = useEditorStore((s) => s.scenario?.http.method ?? "");
  const url = useEditorStore((s) => s.scenario?.http.url ?? "");
  const sending = useResponseStore((s) => s.sending);
  return (
    <div
      role="alert"
      className="flex animate-in flex-col gap-2 rounded-lg border border-danger/30 bg-danger/5 p-3 duration-150 fade-in-0"
    >
      <div className="flex items-center gap-2">
        <p className="text-sm font-medium text-danger">Request failed</p>
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto"
          disabled={sending}
          onClick={() => void sendCurrentScenario()}
        >
          <RotateCw /> Send again
        </Button>
      </div>
      <p className="flex min-w-0 items-center gap-2 font-mono text-xs text-muted-foreground">
        <MethodBadge method={method} />
        <span className="truncate">{url}</span>
      </p>
      <pre className="font-mono text-xs whitespace-pre-wrap text-danger">{error}</pre>
    </div>
  );
}

function NothingSentYet() {
  return (
    <div className="grid h-full animate-in place-items-center p-6 text-center text-sm text-muted-foreground duration-150 fade-in-0">
      <div>
        <div className="mx-auto mb-3 size-14 rounded-full bg-[radial-gradient(closest-side,var(--color-brand-2),transparent)] opacity-30" />
        Press{" "}
        {shortcutKeys("send").map((key) => (
          <kbd key={key} className="mx-0.5 rounded border px-1.5 py-0.5 text-xs">
            {key}
          </kbd>
        ))}{" "}
        to send
      </div>
    </div>
  );
}

function ChecksTabLabel({ response }: { response: HttpResponse }) {
  const failed = response.checks.filter((check) => !check.ok).length;
  return (
    <>
      Checks
      <span className={cn("ml-1 text-xs tabular-nums", failed ? "text-danger" : "text-success")}>
        {response.checks.length - failed}/{response.checks.length}
      </span>
    </>
  );
}

/** The response's tabs; with no response (the send failed), only History. */
function ResponseTabs({ response, historyAt }: { response: HttpResponse | null; historyAt: number | null }) {
  const historyCount = useResponseStore((s) => s.history.length);
  return (
    <Tabs defaultValue={response ? "body" : "history"} className="min-h-0 flex-1 gap-3">
      <TabsList variant="line" className="h-8 gap-2">
        {response && (
          <>
            <TabsTrigger value="body">Body</TabsTrigger>
            <TabsTrigger value="headers">
              Headers
              <CountBadge count={response.headers.length} />
            </TabsTrigger>
            {response.checks.length > 0 && (
              <TabsTrigger value="checks">
                <ChecksTabLabel response={response} />
              </TabsTrigger>
            )}
          </>
        )}
        <TabsTrigger value="history">
          History
          <CountBadge count={historyCount} />
        </TabsTrigger>
      </TabsList>
      {response && (
        <>
          <TabsContent value="body" className="min-h-0">
            {/* Remount per response so search and view state start fresh. */}
            <BodyViewer key={`${response.url}:${historyAt}:${response.timeMs}`} response={response} />
          </TabsContent>
          <TabsContent value="headers" className="min-h-0 overflow-auto">
            <HeadersTable response={response} />
          </TabsContent>
          <TabsContent value="checks" className="min-h-0 overflow-auto">
            <ChecksList checks={response.checks} />
          </TabsContent>
        </>
      )}
      <TabsContent value="history" className="min-h-0 overflow-auto">
        <HistoryList />
      </TabsContent>
    </Tabs>
  );
}

export function ResponsePanel() {
  const response = useResponseStore((s) => s.response);
  const error = useResponseStore((s) => s.error);
  const historyAt = useResponseStore((s) => s.historyAt);
  const sending = useResponseStore((s) => s.sending);

  if (!response && !error && !sending) return <NothingSentYet />;
  return (
    <div className="flex h-full flex-col gap-3 p-4">
      {sending && <SendingBar />}
      {error && <RequestFailed error={error} />}
      {/* The last response stays, dimmed, until the new one arrives. */}
      <div className={cn("flex min-h-0 flex-1 flex-col gap-3 transition-opacity", sending && "opacity-50")}>
        {response && (
          // Remount per response so it fades in on every send or history pick.
          <div key={`${historyAt}:${response.timeMs}`} className="animate-in duration-150 fade-in-0">
            <ResponseSummary response={response} historyAt={historyAt} />
          </div>
        )}
        {(response || error) && (
          <ResponseTabs key={response ? "response" : "failed"} response={response} historyAt={historyAt} />
        )}
      </div>
    </div>
  );
}
