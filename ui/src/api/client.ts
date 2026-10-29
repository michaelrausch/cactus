/**
 * The UI's view of the backend. Types come from the generated bindings; the
 * `_Serialize` variants are what the backend sends, which is also what the UI
 * holds in state and sends back.
 */
import type * as Bindings from "./bindings";

export { commands as backend, events as backendEvents } from "./bindings";

export type Auth = Bindings.Auth_Serialize;
export type BodyType = Bindings.BodyType;
export type Check = Bindings.Check;
export type EnvFile = Bindings.EnvFile;
export type Expect = Bindings.Expect_Serialize;
export type HistoryEntry = Bindings.HistoryEntry_Serialize;
export type HttpResponse = Bindings.Response_Serialize;
export type ImportSummary = Bindings.ImportSummary;
export type JsonValue = Bindings.JsonValue;
export type RunResult = Bindings.RunResult_Serialize;
export type Scenario = Bindings.Scenario_Serialize;
export type ScenarioEntry = Bindings.ScenarioEntry;
export type ScenarioPath = Bindings.ScenarioPath;
export type StatusExpect = Bindings.StatusExpect;

export type Env = Record<string, string>;
