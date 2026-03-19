/**
 * Unit tests for cross_project_guide semantic keyword detection
 * and filterCrossProjectPaths path filtering.
 */
import { describe, it, expect } from "vitest";
import { CROSS_PROJECT_KEYWORDS, filterCrossProjectPaths } from "../index.js";
function detectsCrossProject(planText) {
    const regex = new RegExp(CROSS_PROJECT_KEYWORDS.join("|"), "i");
    return regex.test(planText);
}
describe("CROSS_PROJECT_KEYWORDS — new semantic keywords", () => {
    it("detects 'separate repo'", () => {
        expect(detectsCrossProject("We will create a separate repo for this service.")).toBe(true);
    });
    it("detects 'separate repository'", () => {
        expect(detectsCrossProject("Deploy to a separate repository called reefagent-mcp-jira.")).toBe(true);
    });
    it("detects 'new repo'", () => {
        expect(detectsCrossProject("Create a new repo under the organization.")).toBe(true);
    });
    it("detects 'new repository'", () => {
        expect(detectsCrossProject("Initialize a new repository for the plugin.")).toBe(true);
    });
    it("detects 'different codebase'", () => {
        expect(detectsCrossProject("The agent lives in a different codebase.")).toBe(true);
    });
    it("detects 'other codebase'", () => {
        expect(detectsCrossProject("Changes also needed in the other codebase.")).toBe(true);
    });
    it("detects 'monorepo boundary'", () => {
        expect(detectsCrossProject("This crosses a monorepo boundary.")).toBe(true);
    });
    it("detects 'external package'", () => {
        expect(detectsCrossProject("Publish as an external package on npm.")).toBe(true);
    });
    it("detects 'external module'", () => {
        expect(detectsCrossProject("Import from an external module.")).toBe(true);
    });
    it("is case-insensitive for new keywords", () => {
        expect(detectsCrossProject("Create a Separate Repo for the mcp server.")).toBe(true);
        expect(detectsCrossProject("NEW REPOSITORY setup required.")).toBe(true);
        expect(detectsCrossProject("DIFFERENT CODEBASE integration.")).toBe(true);
    });
});
describe("CROSS_PROJECT_KEYWORDS — regression: existing keywords still detected", () => {
    it("detects 'cross-project'", () => {
        expect(detectsCrossProject("This is a cross-project task.")).toBe(true);
    });
    it("detects 'cross project' (no hyphen)", () => {
        expect(detectsCrossProject("cross project orchestration needed.")).toBe(true);
    });
    it("detects 'multi-project'", () => {
        expect(detectsCrossProject("multi-project plan involving two repos.")).toBe(true);
    });
    it("detects 'target project'", () => {
        expect(detectsCrossProject("The target project is reefbot.ai.")).toBe(true);
    });
    it("detects 'another project'", () => {
        expect(detectsCrossProject("Execute steps in another project.")).toBe(true);
    });
    it("detects 'different project'", () => {
        expect(detectsCrossProject("This belongs to a different project.")).toBe(true);
    });
    it("detects 'project b'", () => {
        expect(detectsCrossProject("Work will happen in project b alongside project a.")).toBe(true);
    });
    it("does not trigger on unrelated text", () => {
        expect(detectsCrossProject("Refactor the authentication service.")).toBe(false);
        expect(detectsCrossProject("Fix the login flow bug.")).toBe(false);
        expect(detectsCrossProject("Add unit tests for the repository layer.")).toBe(false);
    });
});
describe("filterCrossProjectPaths", () => {
    const root = "/Users/lazabogdan/Code/ralphx";
    it("returns empty array when all paths are within the project", () => {
        const paths = [
            "/Users/lazabogdan/Code/ralphx/src/index.ts",
            "/Users/lazabogdan/Code/ralphx/src-tauri/main.rs",
        ];
        expect(filterCrossProjectPaths(paths, root)).toEqual([]);
    });
    it("returns only out-of-project paths when mixed", () => {
        const paths = [
            "/Users/lazabogdan/Code/ralphx/src/index.ts",
            "/Users/lazabogdan/Code/other-project/src/main.ts",
        ];
        expect(filterCrossProjectPaths(paths, root)).toEqual([
            "/Users/lazabogdan/Code/other-project/src/main.ts",
        ]);
    });
    it("returns all paths when projectWorkingDir is null", () => {
        const paths = [
            "/Users/lazabogdan/Code/ralphx/src/index.ts",
            "/Users/lazabogdan/Code/other/main.ts",
        ];
        expect(filterCrossProjectPaths(paths, null)).toEqual(paths);
    });
    it("filters out path that exactly equals the project root", () => {
        expect(filterCrossProjectPaths([root], root)).toEqual([]);
    });
    it("filters out path equal to root with trailing slash", () => {
        expect(filterCrossProjectPaths([root + "/"], root)).toEqual([]);
    });
    it("does not filter similar-prefix directory (e.g. ralphx-other)", () => {
        const paths = ["/Users/lazabogdan/Code/ralphx-other/src/main.ts"];
        expect(filterCrossProjectPaths(paths, root)).toEqual(paths);
    });
    it("does not filter relative ../paths (no project root match)", () => {
        const paths = ["../some-sibling/file.ts"];
        expect(filterCrossProjectPaths(paths, root)).toEqual(paths);
    });
    it("handles project root with trailing slash correctly", () => {
        const paths = [
            "/Users/lazabogdan/Code/ralphx/src/index.ts",
            "/Users/lazabogdan/Code/other/main.ts",
        ];
        expect(filterCrossProjectPaths(paths, root + "/")).toEqual([
            "/Users/lazabogdan/Code/other/main.ts",
        ]);
    });
});
//# sourceMappingURL=cross-project-guide.test.js.map