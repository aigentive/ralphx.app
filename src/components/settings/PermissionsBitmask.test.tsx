/**
 * Tests for PermissionsBitmask component
 *
 * Covers: rendering active/inactive pills, toggle interaction,
 * disabled state, readOnly state.
 */

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { PermissionsBitmask } from "./PermissionsBitmask";
import { PERM_READ, PERM_WRITE, PERM_ADMIN } from "@/types/api-key";

describe("PermissionsBitmask", () => {
  describe("rendering", () => {
    it("renders all three permission pills", () => {
      render(<PermissionsBitmask value={0} onChange={vi.fn()} />);

      expect(screen.getByTestId("perm-toggle-read")).toBeInTheDocument();
      expect(screen.getByTestId("perm-toggle-write")).toBeInTheDocument();
      expect(screen.getByTestId("perm-toggle-admin")).toBeInTheDocument();
    });

    it("shows the permissions-bitmask container", () => {
      render(<PermissionsBitmask value={0} onChange={vi.fn()} />);
      expect(screen.getByTestId("permissions-bitmask")).toBeInTheDocument();
    });

    it("shows Read as active when PERM_READ bit is set", () => {
      render(<PermissionsBitmask value={PERM_READ} onChange={vi.fn()} />);
      const readBtn = screen.getByTestId("perm-toggle-read");
      // Active buttons have accent color class
      expect(readBtn.className).toMatch(/accent-primary/);
    });

    it("shows Write as active when PERM_WRITE bit is set", () => {
      render(<PermissionsBitmask value={PERM_WRITE} onChange={vi.fn()} />);
      const writeBtn = screen.getByTestId("perm-toggle-write");
      expect(writeBtn.className).toMatch(/accent-primary/);
    });

    it("shows Admin as active when PERM_ADMIN bit is set", () => {
      render(<PermissionsBitmask value={PERM_ADMIN} onChange={vi.fn()} />);
      const adminBtn = screen.getByTestId("perm-toggle-admin");
      expect(adminBtn.className).toMatch(/accent-primary/);
    });

    it("shows all three active when value=7 (all bits set)", () => {
      render(<PermissionsBitmask value={7} onChange={vi.fn()} />);
      expect(screen.getByTestId("perm-toggle-read").className).toMatch(/accent-primary/);
      expect(screen.getByTestId("perm-toggle-write").className).toMatch(/accent-primary/);
      expect(screen.getByTestId("perm-toggle-admin").className).toMatch(/accent-primary/);
    });

    it("shows all inactive when value=0", () => {
      render(<PermissionsBitmask value={0} onChange={vi.fn()} />);
      expect(screen.getByTestId("perm-toggle-read").className).not.toMatch(/accent-primary/);
      expect(screen.getByTestId("perm-toggle-write").className).not.toMatch(/accent-primary/);
      expect(screen.getByTestId("perm-toggle-admin").className).not.toMatch(/accent-primary/);
    });
  });

  describe("toggle interaction", () => {
    it("calls onChange with XOR result when Read clicked", () => {
      const onChange = vi.fn();
      render(<PermissionsBitmask value={0} onChange={onChange} />);

      fireEvent.click(screen.getByTestId("perm-toggle-read"));

      expect(onChange).toHaveBeenCalledWith(PERM_READ);
    });

    it("calls onChange with XOR result when Write clicked", () => {
      const onChange = vi.fn();
      render(<PermissionsBitmask value={0} onChange={onChange} />);

      fireEvent.click(screen.getByTestId("perm-toggle-write"));

      expect(onChange).toHaveBeenCalledWith(PERM_WRITE);
    });

    it("calls onChange with XOR result when Admin clicked", () => {
      const onChange = vi.fn();
      render(<PermissionsBitmask value={0} onChange={onChange} />);

      fireEvent.click(screen.getByTestId("perm-toggle-admin"));

      expect(onChange).toHaveBeenCalledWith(PERM_ADMIN);
    });

    it("toggles off when active bit clicked", () => {
      const onChange = vi.fn();
      render(<PermissionsBitmask value={PERM_READ} onChange={onChange} />);

      fireEvent.click(screen.getByTestId("perm-toggle-read"));

      // XOR: PERM_READ ^ PERM_READ = 0
      expect(onChange).toHaveBeenCalledWith(0);
    });
  });

  describe("disabled state", () => {
    it("does not call onChange when disabled and pill clicked", () => {
      const onChange = vi.fn();
      render(<PermissionsBitmask value={0} onChange={onChange} disabled />);

      fireEvent.click(screen.getByTestId("perm-toggle-read"));

      expect(onChange).not.toHaveBeenCalled();
    });

    it("pills have disabled attribute when disabled", () => {
      render(<PermissionsBitmask value={0} onChange={vi.fn()} disabled />);

      expect(screen.getByTestId("perm-toggle-read")).toBeDisabled();
      expect(screen.getByTestId("perm-toggle-write")).toBeDisabled();
      expect(screen.getByTestId("perm-toggle-admin")).toBeDisabled();
    });
  });

  describe("readOnly state", () => {
    it("does not call onChange when readOnly and pill clicked", () => {
      const onChange = vi.fn();
      render(<PermissionsBitmask value={PERM_READ} onChange={onChange} readOnly />);

      fireEvent.click(screen.getByTestId("perm-toggle-read"));

      expect(onChange).not.toHaveBeenCalled();
    });
  });
});
