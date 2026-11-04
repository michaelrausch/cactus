import { CountBadge } from "@/components/Badges";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import { useResponseStore } from "@/stores/responseStore";
import type { HttpResponse } from "@/api/client";
import { BodyViewer } from "./BodyViewer";
import { ChecksList } from "./ChecksList";
import { HeadersTable } from "./HeadersTable";
import { HistoryList } from "./HistoryList";
import { ResponseSummary } from "./ResponseSummary";

function RequestFailed({ error }: { error: string }) {
  return (
    <div className="p-4">
      <p className="mb-2 text-xs font-medium text-rose-400">Request failed</p>
      <pre className="font-mono text-xs whitespace-pre-wrap text-rose-300/90">{error}</pre>
    </div>
  );
}

function NothingSentYet() {
  return (
    <div className="grid h-full place-items-center p-6 text-center text-sm text-muted-foreground">
      <div>
        <div className="mx-auto mb-3 size-14 rounded-full bg-[radial-gradient(closest-side,var(--color-brand-2),transparent)] opacity-30" />
        Press <kbd className="rounded border px-1.5 py-0.5 text-xs">Ctrl</kbd>{" "}
        <kbd className="rounded border px-1.5 py-0.5 text-xs">Enter</kbd> to send
      </div>
    </div>
  );
}

function ChecksTabLabel({ response }: { response: HttpResponse }) {
  const failed = response.checks.filter((check) => !check.ok).length;
  return (
    <>
      Checks
      <span className={cn("ml-1 text-[11px] tabular-nums", failed ? "text-rose-400" : "text-emerald-400")}>
        {response.checks.length - failed}/{response.checks.length}
      </span>
    </>
  );
}

function ResponseTabs({ response, historyAt }: { response: HttpResponse; historyAt: number | null }) {
  const historyCount = useResponseStore((s) => s.history.length);
  return (
    <Tabs defaultValue="body" className="min-h-0 flex-1 gap-3">
      <TabsList variant="line" className="h-8 gap-2">
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
        <TabsTrigger value="history">
          History
          <CountBadge count={historyCount} />
        </TabsTrigger>
      </TabsList>
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

  if (error) return <RequestFailed error={error} />;
  if (!response) return <NothingSentYet />;
  return (
    <div className="flex h-full flex-col gap-3 p-4">
      <ResponseSummary response={response} historyAt={historyAt} />
      <ResponseTabs response={response} historyAt={historyAt} />
    </div>
  );
}
