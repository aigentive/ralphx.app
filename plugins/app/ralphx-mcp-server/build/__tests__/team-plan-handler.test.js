import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { handleRequestTeamPlan } from "../team-plan-handler.js";
describe("handleRequestTeamPlan", () => {
    const tempDirs = [];
    afterEach(() => {
        vi.restoreAllMocks();
        vi.unstubAllGlobals();
        for (const dir of tempDirs.splice(0)) {
            fs.rmSync(dir, { recursive: true, force: true });
        }
    });
    function writeTeamConfig(contents) {
        const homeDir = fs.mkdtempSync(path.join(os.tmpdir(), "ralphx-team-plan-"));
        tempDirs.push(homeDir);
        vi.spyOn(os, "homedir").mockReturnValue(homeDir);
        const configDir = path.join(homeDir, ".claude", "teams", "alpha-team");
        fs.mkdirSync(configDir, { recursive: true });
        fs.writeFileSync(path.join(configDir, "config.json"), `${JSON.stringify(contents)}\n`, "utf8");
    }
    it("omits invalid lead_session_id values loaded from team config", async () => {
        writeTeamConfig({ leadSessionId: "../../escape" });
        const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
            success: true,
            plan_id: "plan-1",
            message: "auto approved",
            auto_approved: true,
            teammates_spawned: [],
        }), { status: 200 }));
        vi.stubGlobal("fetch", fetchMock);
        const result = await handleRequestTeamPlan({
            process: "ideation",
            teammates: [],
            team_name: "alpha-team",
        }, "ideation", "context-1", undefined);
        expect(result.isError).toBeUndefined();
        expect(fetchMock).toHaveBeenCalledTimes(1);
        expect(fetchMock).toHaveBeenCalledWith("http://127.0.0.1:3847/api/team/plan/request", expect.objectContaining({
            body: expect.not.stringContaining("../../escape"),
        }));
    });
    it("passes through validated lead_session_id values from team config", async () => {
        writeTeamConfig({ leadSessionId: "lead-session_01" });
        const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
            success: true,
            plan_id: "plan-1",
            message: "auto approved",
            auto_approved: true,
            teammates_spawned: [],
        }), { status: 200 }));
        vi.stubGlobal("fetch", fetchMock);
        await handleRequestTeamPlan({
            process: "ideation",
            teammates: [],
            team_name: "alpha-team",
        }, "ideation", "context-1", undefined);
        expect(fetchMock).toHaveBeenCalledWith("http://127.0.0.1:3847/api/team/plan/request", expect.objectContaining({
            body: expect.stringContaining('"lead_session_id":"lead-session_01"'),
        }));
    });
});
//# sourceMappingURL=team-plan-handler.test.js.map