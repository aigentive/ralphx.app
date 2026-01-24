/**
 * Tests for design system CSS variables
 *
 * Verifies that all required design tokens are defined
 * in the global styles.
 */

import { describe, it, expect, beforeAll } from "vitest";
import fs from "fs";
import path from "path";

describe("design-tokens", () => {
  let cssContent: string;

  beforeAll(() => {
    const cssPath = path.resolve(__dirname, "./globals.css");
    cssContent = fs.readFileSync(cssPath, "utf-8");
  });

  describe("color palette", () => {
    it("should define background colors", () => {
      expect(cssContent).toContain("--bg-base:");
      expect(cssContent).toContain("--bg-surface:");
      expect(cssContent).toContain("--bg-elevated:");
      expect(cssContent).toContain("--bg-hover:");
    });

    it("should define text colors", () => {
      expect(cssContent).toContain("--text-primary:");
      expect(cssContent).toContain("--text-secondary:");
      expect(cssContent).toContain("--text-muted:");
    });

    it("should define accent colors (warm, NOT purple)", () => {
      expect(cssContent).toContain("--accent-primary:");
      expect(cssContent).toContain("--accent-secondary:");
      // Verify warm orange, not purple
      expect(cssContent).toMatch(/--accent-primary:\s*#ff6b35/);
    });

    it("should define status colors", () => {
      expect(cssContent).toContain("--status-success:");
      expect(cssContent).toContain("--status-warning:");
      expect(cssContent).toContain("--status-error:");
      expect(cssContent).toContain("--status-info:");
    });

    it("should define border colors", () => {
      expect(cssContent).toContain("--border-subtle:");
      expect(cssContent).toContain("--border-default:");
    });
  });

  describe("typography", () => {
    it("should define font families (NOT Inter)", () => {
      expect(cssContent).toContain("--font-display:");
      expect(cssContent).toContain("--font-body:");
      expect(cssContent).toContain("--font-mono:");
      // Verify NOT using Inter
      expect(cssContent).not.toMatch(/--font-display:.*Inter/);
      expect(cssContent).not.toMatch(/--font-body:.*Inter/);
    });

    it("should define font sizes", () => {
      expect(cssContent).toContain("--text-xs:");
      expect(cssContent).toContain("--text-sm:");
      expect(cssContent).toContain("--text-base:");
      expect(cssContent).toContain("--text-lg:");
      expect(cssContent).toContain("--text-xl:");
    });
  });

  describe("spacing (8pt grid)", () => {
    it("should define spacing scale", () => {
      expect(cssContent).toContain("--space-1:");
      expect(cssContent).toContain("--space-2:");
      expect(cssContent).toContain("--space-3:");
      expect(cssContent).toContain("--space-4:");
      expect(cssContent).toContain("--space-6:");
      expect(cssContent).toContain("--space-8:");
      expect(cssContent).toContain("--space-12:");
    });

    it("should use 8pt grid values", () => {
      // space-1 = 4px, space-2 = 8px, etc.
      expect(cssContent).toMatch(/--space-1:\s*4px/);
      expect(cssContent).toMatch(/--space-2:\s*8px/);
      expect(cssContent).toMatch(/--space-4:\s*16px/);
      expect(cssContent).toMatch(/--space-8:\s*32px/);
    });
  });

  describe("other tokens", () => {
    it("should define border radius", () => {
      expect(cssContent).toContain("--radius-sm:");
      expect(cssContent).toContain("--radius-md:");
      expect(cssContent).toContain("--radius-lg:");
    });

    it("should define shadows", () => {
      expect(cssContent).toContain("--shadow-sm:");
      expect(cssContent).toContain("--shadow-md:");
      expect(cssContent).toContain("--shadow-lg:");
    });

    it("should define transitions", () => {
      expect(cssContent).toContain("--transition-fast:");
      expect(cssContent).toContain("--transition-normal:");
    });
  });

  describe("anti-AI-slop guardrails", () => {
    it("should NOT use purple gradients", () => {
      // Check no purple hex codes in accents
      expect(cssContent).not.toMatch(/--accent.*#[0-9a-f]*[8-9a-f][0-9a-f][0-9a-f][0-9a-f]ff/i);
    });

    it("should use dark grays, NOT pure black", () => {
      // bg-base should be dark gray, not #000
      expect(cssContent).toMatch(/--bg-base:\s*#0f0f0f/);
    });

    it("should use off-white, NOT pure white", () => {
      // text-primary should be off-white, not #fff
      expect(cssContent).toMatch(/--text-primary:\s*#f0f0f0/);
    });
  });
});
