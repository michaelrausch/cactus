/**
 * The UI's view of the backend. Types come from the generated bindings; the
 * `_Serialize` variants are what the backend sends, which is also what the UI
 * holds in state and sends back.
 */
import type * as Bindings from "./bindings";

export { commands as backend, events as backendEvents } from "./bindings";

export type Action = Bindings.Action_Serialize;
export type AlertEvent = Bindings.AlertEvent_Serialize;
export type AlertOn = Bindings.AlertOn;
export type ContextEntry = Bindings.ContextEntry;
export type Critter = Bindings.Critter_Serialize;
export type CritterSummary = Bindings.CritterSummary_Serialize;
export type Flow = Bindings.Flow_Serialize;
export type FlowParse = Bindings.FlowParse_Serialize;
export type FlowRun = Bindings.FlowRun_Serialize;
export type FlowStep = Bindings.FlowStep_Serialize;
export type FlowStepResult = Bindings.FlowStepResult_Serialize;
export type FlowStepStart = Bindings.FlowStepStart;
export type FlowSummary = Bindings.FlowSummary_Serialize;
export type MatrixCell = Bindings.MatrixCell_Serialize;
export type MonitorAlert = Bindings.MonitorAlert_Serialize;
export type RunRecord = Bindings.RunRecord_Serialize;
export type StepRecord = Bindings.StepRecord_Serialize;
export type Schedule = Bindings.Schedule_Serialize;
export type ScheduleState = Bindings.ScheduleState_Serialize;
export type Trigger = Bindings.Trigger_Serialize;
export type ScheduleStates = Bindings.ScheduleStates_Serialize;
export type SecretsInfo = Bindings.SecretsInfo_Serialize;
export type SecretsStatus = Bindings.SecretsStatus;
export type Species = Bindings.Species;
export type VarSource = Bindings.VarSource;
export type Assertion = Bindings.Assertion_Serialize;
export type Auth = Bindings.Auth_Serialize;
export type AuthMode = Bindings.AuthMode_Serialize;
export type Body = Bindings.Body_Serialize;
export type BranchInfo = Bindings.BranchInfo_Serialize;
export type ChangeKind = Bindings.ChangeKind;
export type Check = Bindings.Check;
export type CommitSummary = Bindings.CommitSummary;
export type DiffLine = Bindings.DiffLine_Serialize;
export type DiffSide = Bindings.DiffSide;
export type EnvFile = Bindings.EnvFile;
export type FileDiff = Bindings.FileDiff_Serialize;
export type FileStatus = Bindings.FileStatus_Serialize;
export type FormField = Bindings.FormField_Serialize;
export type GitIdentity = Bindings.GitIdentity;
export type GitStatus = Bindings.GitStatus_Serialize;
export type Header = Bindings.Header_Serialize;
export type HistoryEntry = Bindings.HistoryEntry_Serialize;
export type Http = Bindings.Http_Serialize;
export type HttpResponse = Bindings.Response_Serialize;
export type ImportSummary = Bindings.ImportSummary;
export type JsonValue = Bindings.JsonValue;
export type MultipartField = Bindings.MultipartField_Serialize;
export type Param = Bindings.Param_Serialize;
export type RunResult = Bindings.RunResult_Serialize;
export type Scenario = Bindings.Scenario_Serialize;
export type ScenarioEntry = Bindings.ScenarioEntry;
export type ProjectPath = Bindings.ProjectPath;
export type Script = Bindings.Script_Serialize;
export type SetVariable = Bindings.SetVariable_Serialize;
export type TypedBody = Bindings.TypedBody_Serialize;

export type Env = Record<string, string>;
