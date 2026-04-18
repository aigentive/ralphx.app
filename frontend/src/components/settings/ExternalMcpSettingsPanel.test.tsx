/**
 * ExternalMcpSettingsPanel Tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ExternalMcpSettingsPanel } from "./ExternalMcpSettingsPanel";
import * as externalMcpApi from "@/api/external-mcp";
import type { ExternalMcpConfigView } from "@/api/external-mcp";

vi.mock("@/api/external-mcp", () => ({
  getExternalMcpConfig: vi.fn(),
  updateExternalMcpConfig: vi.fn(),
}));

const defaultConfig: ExternalMcpConfigView = {
  enabled: false,
  port: 3848,
  host: "127.0.0.1",
  authToken: null,
  nodePath: null,
};

const configWithToken: ExternalMcpConfigView = {
  ...defaultConfig,
  authToken: "••••••••",
};

describe("ExternalMcpSettingsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(externalMcpApi.getExternalMcpConfig).mockResolvedValue(defaultConfig);
    vi.mocked(externalMcpApi.updateExternalMcpConfig).mockResolvedValue(undefined);
  });

  it("renders the panel title", async () => {
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect(screen.getByText("External MCP")).toBeInTheDocument();
    });
  });

  it("renders all fields after loading", async () => {
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect(screen.getByTestId("ext-mcp-enabled")).toBeInTheDocument();
      expect(screen.getByTestId("ext-mcp-host")).toBeInTheDocument();
      expect(screen.getByTestId("ext-mcp-port")).toBeInTheDocument();
      expect(screen.getByTestId("ext-mcp-auth-token")).toBeInTheDocument();
      expect(screen.getByTestId("ext-mcp-node-path")).toBeInTheDocument();
    });
  });

  it("renders a single card-level restart-required notice", async () => {
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      const badges = screen.getAllByTestId("restart-required-badge");
      expect(badges.length).toBe(1);
    });
  });

  it("shows masked placeholder when auth token is set", async () => {
    vi.mocked(externalMcpApi.getExternalMcpConfig).mockResolvedValue(configWithToken);

    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      const authInput = screen.getByTestId("ext-mcp-auth-token") as HTMLInputElement;
      // Value should be the mask constant, not empty
      expect(authInput.value).toBe("••••••••");
    });
  });

  it("shows empty auth token field when token is not set", async () => {
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      const authInput = screen.getByTestId("ext-mcp-auth-token") as HTMLInputElement;
      expect(authInput.value).toBe("");
    });
  });

  it("auth token field is type=password", async () => {
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      const authInput = screen.getByTestId("ext-mcp-auth-token") as HTMLInputElement;
      expect(authInput.type).toBe("password");
    });
  });

  it("populates form fields from loaded config", async () => {
    vi.mocked(externalMcpApi.getExternalMcpConfig).mockResolvedValue({
      ...defaultConfig,
      enabled: true,
      port: 4000,
      host: "0.0.0.0",
      nodePath: "/usr/bin/node",
    });

    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect((screen.getByTestId("ext-mcp-port") as HTMLInputElement).value).toBe("4000");
      expect((screen.getByTestId("ext-mcp-host") as HTMLInputElement).value).toBe("0.0.0.0");
      expect((screen.getByTestId("ext-mcp-node-path") as HTMLInputElement).value).toBe(
        "/usr/bin/node"
      );
    });
  });

  it("calls updateExternalMcpConfig with changed fields only on save", async () => {
    const user = userEvent.setup();
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect(screen.getByTestId("ext-mcp-host")).toBeInTheDocument();
    });

    const hostInput = screen.getByTestId("ext-mcp-host");
    await user.clear(hostInput);
    await user.type(hostInput, "0.0.0.0");

    await user.click(screen.getByTestId("ext-mcp-save"));

    await waitFor(() => {
      expect(externalMcpApi.updateExternalMcpConfig).toHaveBeenCalledWith(
        expect.objectContaining({ host: "0.0.0.0" })
      );
    });
  });

  it("does not send authToken when it still shows the mask", async () => {
    const user = userEvent.setup();
    vi.mocked(externalMcpApi.getExternalMcpConfig).mockResolvedValue(configWithToken);

    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect(screen.getByTestId("ext-mcp-save")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("ext-mcp-save"));

    await waitFor(() => {
      const call = vi.mocked(externalMcpApi.updateExternalMcpConfig).mock.calls[0]?.[0];
      expect(call).not.toHaveProperty("authToken");
    });
  });

  it("shows success feedback after saving", async () => {
    const user = userEvent.setup();
    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect(screen.getByTestId("ext-mcp-save")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("ext-mcp-save"));

    await waitFor(() => {
      expect(screen.getByText(/restart the app to apply/i)).toBeInTheDocument();
    });
  });

  it("shows error feedback when save fails", async () => {
    const user = userEvent.setup();
    vi.mocked(externalMcpApi.updateExternalMcpConfig).mockRejectedValue(
      new Error("Permission denied")
    );

    render(<ExternalMcpSettingsPanel />);

    await waitFor(() => {
      expect(screen.getByTestId("ext-mcp-save")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("ext-mcp-save"));

    await waitFor(() => {
      expect(screen.getByText(/permission denied/i)).toBeInTheDocument();
    });
  });
});
