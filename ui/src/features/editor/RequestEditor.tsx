import { ActiveDot, CountBadge } from "@/components/Badges";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { checkCount } from "@/domain/expect";
import { parseQuery } from "@/domain/query";
import { AuthTab } from "./tabs/AuthTab";
import { BodyTab } from "./tabs/BodyTab";
import { CaptureTab } from "./tabs/CaptureTab";
import { ChecksTab } from "./tabs/ChecksTab";
import { DocsTab } from "./tabs/DocsTab";
import { HeadersTab } from "./tabs/HeadersTab";
import { ParamsTab } from "./tabs/ParamsTab";
import type { ScenarioTabProps } from "./types";
import { UrlBar } from "./UrlBar";
import { useBodyDraft } from "./useBodyDraft";

function TitleField({ scenario, onChange }: ScenarioTabProps) {
  return (
    <div>
      <input
        value={scenario.title}
        onChange={(e) => onChange({ ...scenario, title: e.target.value })}
        placeholder="Untitled request"
        className="w-full bg-transparent text-lg font-semibold tracking-tight outline-none placeholder:text-muted-foreground"
      />
      {scenario.description && (
        <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">{scenario.description}</p>
      )}
    </div>
  );
}

/**
 * Editor for one scenario. Mount it with a `key` that changes when the
 * scenario is switched or reloaded, so local drafts start fresh.
 */
export function RequestEditor(props: ScenarioTabProps) {
  const { scenario } = props;
  const bodyDraft = useBodyDraft(props);

  return (
    <div className="flex h-full flex-col gap-5 px-6 pt-4 pb-4">
      <TitleField {...props} />
      <UrlBar {...props} />
      <Tabs defaultValue="params" className="min-h-0 flex-1 gap-4">
        <TabsList variant="line" className="h-8 gap-2">
          <TabsTrigger value="params">
            Params
            <CountBadge count={parseQuery(scenario.url).length} />
          </TabsTrigger>
          <TabsTrigger value="headers">
            Headers
            <CountBadge count={Object.keys(scenario.headers).length} />
          </TabsTrigger>
          <TabsTrigger value="auth">
            Auth
            <ActiveDot visible={!!scenario.auth} />
          </TabsTrigger>
          <TabsTrigger value="body">
            Body
            <ActiveDot visible={bodyDraft.bodyType !== "none"} />
          </TabsTrigger>
          <TabsTrigger value="capture">
            Capture
            <CountBadge count={Object.keys(scenario.capture ?? {}).length} />
          </TabsTrigger>
          <TabsTrigger value="checks">
            Checks
            <CountBadge count={checkCount(scenario.expect)} />
          </TabsTrigger>
          <TabsTrigger value="docs">Docs</TabsTrigger>
        </TabsList>

        <TabsContent value="params" className="overflow-auto">
          <ParamsTab {...props} />
        </TabsContent>
        <TabsContent value="headers" className="overflow-auto">
          <HeadersTab {...props} />
        </TabsContent>
        <TabsContent value="auth" className="flex flex-col gap-4 overflow-auto">
          <AuthTab {...props} />
        </TabsContent>
        <TabsContent value="body" className="flex min-h-0 flex-col gap-2">
          <BodyTab {...props} draft={bodyDraft} />
        </TabsContent>
        <TabsContent value="capture" className="overflow-auto">
          <CaptureTab {...props} />
        </TabsContent>
        <TabsContent value="checks" className="flex flex-col gap-4 overflow-auto">
          <ChecksTab {...props} />
        </TabsContent>
        <TabsContent value="docs" className="flex min-h-0 flex-col">
          <DocsTab {...props} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
