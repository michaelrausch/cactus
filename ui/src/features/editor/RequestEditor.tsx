import { KeyRound } from "lucide-react";
import { ActiveDot, CountBadge } from "@/components/Badges";
import { IconButton } from "@/components/IconButton";
import { KnownNamesContext, VariableDetailsContext } from "@/components/template/knownNames";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { activeAssertionCount, varCount } from "@/domain/assertions";
import { parseQuery } from "@/domain/query";
import { authKind, bodyKind, isAuthRequest, pathParams, withAuthTag } from "@/domain/scenario";
import { useKnownVariables, useVariableDetails } from "@/hooks/useKnownVariables";
import { AssertionsTab } from "./tabs/AssertionsTab";
import { AuthTab } from "./tabs/AuthTab";
import { BodyTab } from "./tabs/BodyTab";
import { DocsTab } from "./tabs/DocsTab";
import { HeadersTab } from "./tabs/HeadersTab";
import { ParamsTab } from "./tabs/ParamsTab";
import { ScriptsTab } from "./tabs/ScriptsTab";
import { OutputsTab } from "./tabs/OutputsTab";
import type { ScenarioTabProps } from "./types";
import { UrlBar } from "./UrlBar";

function TitleField({ scenario, onChange }: ScenarioTabProps) {
  return (
    <input
      value={scenario.info.name}
      onChange={(e) => onChange({ ...scenario, info: { ...scenario.info, name: e.target.value } })}
      placeholder="Untitled request"
      aria-label="Request title"
      className="-ml-1.5 min-w-0 flex-1 rounded-md bg-transparent px-1.5 text-lg font-semibold tracking-tight outline-none placeholder:text-muted-foreground hover:bg-accent/40 focus-visible:bg-accent/60"
    />
  );
}

/** Marks the request as one that logs in (`tags: [auth]`): listed first in the flow builder, keyed in the sidebar. */
function AuthToggle({ scenario, onChange }: ScenarioTabProps) {
  const auth = isAuthRequest(scenario.info.tags);
  const label = auth ? "Logs in (tagged auth). Click to untag" : "Tag as a login request";
  return (
    <IconButton label={label} active={auth} onClick={() => onChange(withAuthTag(scenario, !auth))}>
      <KeyRound />
    </IconButton>
  );
}

/**
 * Editor for one request. Mount it with a `key` that changes when the
 * scenario is switched or reloaded, so local drafts start fresh.
 */
export function RequestEditor(props: ScenarioTabProps) {
  const { scenario } = props;
  const scriptCount = scenario.runtime?.scripts?.length ?? 0;
  const knownNames = useKnownVariables(scenario);
  const variableDetails = useVariableDetails(scenario);

  return (
    <KnownNamesContext value={knownNames}>
      <VariableDetailsContext value={variableDetails}>
        <div className="flex h-full flex-col gap-5 px-6 pt-4 pb-4">
          <div className="flex items-center gap-2">
            <TitleField {...props} />
            <AuthToggle {...props} />
          </div>
          <UrlBar {...props} />
          <Tabs defaultValue="params" className="min-h-0 flex-1 gap-4">
            <TabsList variant="line" className="h-8 gap-2">
              <TabsTrigger value="params">
                Params
                <CountBadge count={parseQuery(scenario.http.url).length + pathParams(scenario).length} />
              </TabsTrigger>
              <TabsTrigger value="headers">
                Headers
                <CountBadge count={(scenario.http.headers ?? []).filter((h) => !h.disabled).length} />
              </TabsTrigger>
              <TabsTrigger value="auth">
                Auth
                <ActiveDot visible={authKind(scenario) !== "none"} />
              </TabsTrigger>
              <TabsTrigger value="body">
                Body
                <ActiveDot visible={bodyKind(scenario) !== "none"} />
              </TabsTrigger>
              <TabsTrigger value="outputs">
                Outputs
                <CountBadge count={varCount(scenario)} />
              </TabsTrigger>
              <TabsTrigger value="assertions">
                Assertions
                <CountBadge count={activeAssertionCount(scenario)} />
              </TabsTrigger>
              {scriptCount > 0 && (
                <TabsTrigger value="scripts">
                  Scripts
                  <CountBadge count={scriptCount} />
                </TabsTrigger>
              )}
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
              <BodyTab {...props} />
            </TabsContent>
            <TabsContent value="outputs" className="overflow-auto">
              <OutputsTab {...props} />
            </TabsContent>
            <TabsContent value="assertions" className="overflow-auto">
              <AssertionsTab {...props} />
            </TabsContent>
            <TabsContent value="scripts" className="flex flex-col gap-4 overflow-auto">
              <ScriptsTab {...props} />
            </TabsContent>
            <TabsContent value="docs" className="flex min-h-0 flex-col">
              <DocsTab {...props} />
            </TabsContent>
          </Tabs>
        </div>
      </VariableDetailsContext>
    </KnownNamesContext>
  );
}
