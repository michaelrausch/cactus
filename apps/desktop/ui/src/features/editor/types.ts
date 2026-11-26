import type { Scenario } from "@/api/client";

/** Every editor tab edits the whole scenario and reports the next version. */
export interface ScenarioTabProps {
  scenario: Scenario;
  onChange: (scenario: Scenario) => void;
}
