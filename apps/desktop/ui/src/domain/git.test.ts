import { describe, expect, it } from "vitest";
import type { FileStatus, GitStatus } from "@/api/client";
import { changeLabel, changesByPath, markerFor, nextSelection, stagedFiles, unstagePaths, unstagedFiles } from "./git";

const file = (path: string, staged: FileStatus["staged"], unstaged: FileStatus["unstaged"]): FileStatus => ({
  path,
  renamedFrom: null,
  staged,
  unstaged,
});

const statusOf = (...files: FileStatus[]): GitStatus => ({
  branch: "main",
  files,
  busy: null,
  stagedOutside: 0,
  lastCommit: null,
});

describe("git status", () => {
  const status = statusOf(file("a.yml", "modified", "modified"), file("b.yml", null, "untracked"));

  it("splits staged and unstaged changes", () => {
    expect(stagedFiles(status).map((f) => f.path)).toEqual(["a.yml"]);
    expect(unstagedFiles(status).map((f) => f.path)).toEqual(["a.yml", "b.yml"]);
  });

  it("marks each file with its newest change", () => {
    const changes = changesByPath(statusOf(file("a.yml", "added", null), file("b.yml", "added", "modified")));
    expect(changes).toEqual({ "a.yml": "added", "b.yml": "modified" });
    expect(changesByPath(null)).toEqual({});
    expect(markerFor("untracked")).toBe("U");
  });

  it("unstages both sides of a rename", () => {
    expect(unstagePaths({ ...file("new.yml", "renamed", null), renamedFrom: "old.yml" })).toEqual([
      "new.yml",
      "old.yml",
    ]);
  });
});

describe("nextSelection", () => {
  it("keeps a selection that still has changes", () => {
    const status = statusOf(file("a.yml", "modified", "modified"));
    expect(nextSelection(status, { path: "a.yml", side: "unstaged" })).toEqual({ path: "a.yml", side: "unstaged" });
  });

  it("follows a file to the side it moved to", () => {
    const status = statusOf(file("b.yml", null, "modified"), file("a.yml", "modified", null));
    expect(nextSelection(status, { path: "a.yml", side: "unstaged" })).toEqual({ path: "a.yml", side: "staged" });
  });

  it("falls back to the first change, or nothing", () => {
    const status = statusOf(file("a.yml", null, "modified"));
    expect(nextSelection(status, { path: "gone.yml", side: "staged" })).toEqual({ path: "a.yml", side: "unstaged" });
    expect(nextSelection(statusOf(), null)).toBeNull();
  });
});

describe("changeLabel", () => {
  it("names requests by title and other files by name", () => {
    const titles = { "auth/login.yml": "Log in" };
    expect(changeLabel("auth/login.yml", titles)).toEqual({ name: "Log in", detail: "auth/login.yml" });
    expect(changeLabel("auth/folder.yml", titles)).toEqual({ name: "folder.yml", detail: "auth" });
  });
});
