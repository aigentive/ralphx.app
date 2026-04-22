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
    // 3-tier token architecture — concatenate all token sources so assertions
    // don't depend on which file a given token currently lives in.
    // See specs/design/styleguide.md.
    const files = [
      "./globals.css",
      "./tokens/primitives.css",
      "./tokens/semantic.css",
      "./tokens/components.css",
      "./themes/light.css",
      "./themes/high-contrast.css",
    ];
    cssContent = files
      .map((f) => fs.readFileSync(path.resolve(__dirname, f), "utf-8"))
      .join("\n");
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
      // Dark theme accent resolves to the brand orange primitive --orange-500
      // which is hsl(14 100% 60%). Verify both links in the chain.
      expect(cssContent).toMatch(/--orange-500:\s*hsl\(14 100% 60%\)/);
      expect(cssContent).toMatch(/--accent-primary:\s*var\(--orange-500\)/);
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
      // --space-* primitives are the direct-CSS scale (1-8). Tailwind's wider
      // 1-16 scale resolves via --spacing-* in the @theme inline block.
      expect(cssContent).toContain("--space-1:");
      expect(cssContent).toContain("--space-2:");
      expect(cssContent).toContain("--space-3:");
      expect(cssContent).toContain("--space-4:");
      expect(cssContent).toContain("--space-6:");
      expect(cssContent).toContain("--space-8:");
      // Tailwind-scale entries available as --spacing-* for utility classes
      expect(cssContent).toContain("--spacing-12:");
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

    it("should keep settings card icon tiles legible in high contrast", () => {
      // HC icon tile pattern: transparent fill + yellow outline + white glyph.
      // Avoids the yellow-on-yellow collision that happens when tinted bg
      // meets accent-colored glyphs. See themes/high-contrast.md §3.
      expect(cssContent).toMatch(/--card-icon-bg:\s*transparent/);
      expect(cssContent).toMatch(/--card-icon-border:\s*var\(--accent-primary\)/);
      expect(cssContent).toMatch(/--card-icon-color:\s*var\(--color-white\)/);
    });
  });

  describe("font-scale contract", () => {
    it("should set 18px as the baseline root font-size", () => {
      expect(cssContent).toMatch(/html\s*\{[^}]*font-size:\s*18px/);
    });

    it("should map lg to an explicit absolute size above the 18px baseline (19.8px)", () => {
      // 110% of 16px (browser default) = 17.6px — SMALLER than 18px baseline.
      // The fix: html[data-font-scale="lg"] must use 19.8px (18 * 1.1).
      expect(cssContent).toMatch(
        /\[data-font-scale="lg"\]\s*\{[^}]*font-size:\s*19\.8px/
      );
    });

    it("should map xl to an explicit absolute size above lg (22.5px)", () => {
      // 125% of 16px = 20px — not relative to the 18px baseline.
      // The fix: html[data-font-scale="xl"] must use 22.5px (18 * 1.25).
      expect(cssContent).toMatch(
        /\[data-font-scale="xl"\]\s*\{[^}]*font-size:\s*22\.5px/
      );
    });

    it("should be monotonic: 18px < 19.8px < 22.5px", () => {
      // Verify the three literal values appear in the file and are monotonic.
      const baselineMatch = cssContent.match(/html\s*\{[^}]*font-size:\s*([\d.]+)px/);
      const lgMatch = cssContent.match(
        /\[data-font-scale="lg"\]\s*\{[^}]*font-size:\s*([\d.]+)px/
      );
      const xlMatch = cssContent.match(
        /\[data-font-scale="xl"\]\s*\{[^}]*font-size:\s*([\d.]+)px/
      );
      expect(baselineMatch).not.toBeNull();
      expect(lgMatch).not.toBeNull();
      expect(xlMatch).not.toBeNull();
      const baseline = parseFloat(baselineMatch![1]);
      const lg = parseFloat(lgMatch![1]);
      const xl = parseFloat(xlMatch![1]);
      expect(lg).toBeGreaterThan(baseline);
      expect(xl).toBeGreaterThan(lg);
    });
  });

  describe("anti-AI-slop guardrails", () => {
    it("should NOT use purple gradients", () => {
      // Check no purple hex codes in accents
      expect(cssContent).not.toMatch(/--accent.*#[0-9a-f]*[8-9a-f][0-9a-f][0-9a-f][0-9a-f]ff/i);
    });

    it("should use dark grays, NOT pure black", () => {
      // Dark theme bg-base should resolve to a dark gray primitive.
      // Primitive --gray-975 is hsl(220 10% 8%) and the semantic layer
      // references it.
      expect(cssContent).toMatch(/--gray-975:\s*hsl\(220 10% 8%\)/);
      expect(cssContent).toMatch(/--bg-base:\s*var\(--gray-975\)/);
    });

    it("should use off-white, NOT pure white", () => {
      // Dark theme text-primary must be off-white (not pure #fff).
      // Currently set directly on :root for dark theme.
      expect(cssContent).toMatch(/--text-primary:\s*hsl\(220 10% 90%\)/);
    });
  });
});
