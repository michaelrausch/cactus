import { THEMES } from "@/theme/themes";

/** What the app counts towards achievements, across every project. */
export type Stat =
  | "requestsSent"
  | "serverErrors"
  | "teapots"
  | "foldersRun"
  | "flowsRun"
  | "requestsCreated"
  | "crittersCreated"
  | "commits"
  | "imports"
  | "projectsOpened"
  | "themesTried";

export type Counts = Partial<Record<Stat, number>>;

export type AchievementIcon =
  | "send"
  | "zap"
  | "rocket"
  | "flame"
  | "server-crash"
  | "teapot"
  | "folder"
  | "workflow"
  | "file-plus"
  | "paw"
  | "users"
  | "git-commit"
  | "download"
  | "compass"
  | "palette"
  | "swatch";

export interface Achievement {
  id: string;
  name: string;
  description: string;
  icon: AchievementIcon;
  stat: Stat;
  /** Unlocked once the stat reaches this. */
  goal: number;
}

export const ACHIEVEMENTS: readonly Achievement[] = [
  {
    id: "requests-1",
    name: "Hello, World",
    description: "Send your first request",
    icon: "send",
    stat: "requestsSent",
    goal: 1,
  },
  {
    id: "requests-100",
    name: "Regular",
    description: "Send 100 requests",
    icon: "zap",
    stat: "requestsSent",
    goal: 100,
  },
  {
    id: "requests-1000",
    name: "Thousandaire",
    description: "Send 1,000 requests",
    icon: "rocket",
    stat: "requestsSent",
    goal: 1000,
  },
  {
    id: "requests-10000",
    name: "Ten Grand",
    description: "Send 10,000 requests",
    icon: "flame",
    stat: "requestsSent",
    goal: 10000,
  },
  {
    id: "server-errors-25",
    name: "It's Not Me",
    description: "Get 25 5xx responses",
    icon: "server-crash",
    stat: "serverErrors",
    goal: 25,
  },
  {
    id: "teapot",
    name: "Short and Stout",
    description: "Get a 418 I'm a teapot",
    icon: "teapot",
    stat: "teapots",
    goal: 1,
  },
  {
    id: "folders-10",
    name: "Batch Job",
    description: "Run 10 folders",
    icon: "folder",
    stat: "foldersRun",
    goal: 10,
  },
  { id: "flows-10", name: "Flow State", description: "Run 10 flows", icon: "workflow", stat: "flowsRun", goal: 10 },
  {
    id: "flows-100",
    name: "Choreographer",
    description: "Run 100 flows",
    icon: "workflow",
    stat: "flowsRun",
    goal: 100,
  },
  {
    id: "requests-created-25",
    name: "Prolific",
    description: "Create 25 requests",
    icon: "file-plus",
    stat: "requestsCreated",
    goal: 25,
  },
  {
    id: "critters-1",
    name: "It's Alive",
    description: "Create your first critter",
    icon: "paw",
    stat: "crittersCreated",
    goal: 1,
  },
  {
    id: "critters-3",
    name: "Critter Keeper",
    description: "Create 3 critters",
    icon: "paw",
    stat: "crittersCreated",
    goal: 3,
  },
  {
    id: "critters-10",
    name: "Zoo",
    description: "Create 10 critters",
    icon: "users",
    stat: "crittersCreated",
    goal: 10,
  },
  {
    id: "commits-10",
    name: "Committed",
    description: "Make 10 commits",
    icon: "git-commit",
    stat: "commits",
    goal: 10,
  },
  {
    id: "first-import",
    name: "Moving In",
    description: "Import a collection",
    icon: "download",
    stat: "imports",
    goal: 1,
  },
  {
    id: "projects-5",
    name: "Explorer",
    description: "Open 5 projects",
    icon: "compass",
    stat: "projectsOpened",
    goal: 5,
  },
  {
    id: "themes-5",
    name: "Window Shopper",
    description: "Try 5 themes",
    icon: "palette",
    stat: "themesTried",
    goal: 5,
  },
  {
    id: "themes-all",
    name: "Chameleon",
    description: "Try every theme",
    icon: "swatch",
    stat: "themesTried",
    goal: THEMES.length,
  },
];

/** Achievements `counts` reaches that aren't in `unlocked` yet. */
export function newlyUnlocked(counts: Counts, unlocked: Record<string, string>): Achievement[] {
  return ACHIEVEMENTS.filter(
    (achievement) => !(achievement.id in unlocked) && (counts[achievement.stat] ?? 0) >= achievement.goal,
  );
}
