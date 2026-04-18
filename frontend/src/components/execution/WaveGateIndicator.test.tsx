/**
 * WaveGateIndicator component tests
 *
 * Tests hasPassedGate logic, wave dot colors, and "Gate Passed" badge.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { WaveGateIndicator } from "./WaveGateIndicator";
import type { TeammateSummary } from "@/api/running-processes";

function mate(name: string, status: string, wave?: number): TeammateSummary {
  return { name, status, ...(wave !== undefined ? { wave } : {}) };
}

describe("WaveGateIndicator", () => {
  describe("hasPassedGate logic", () => {
    it("treats completed teammate as passed", () => {
      const teammates = [mate("a", "completed")];
      render(<WaveGateIndicator currentWave={1} totalWaves={2} teammates={teammates} />);
      // "a" should appear in the passed list (with CheckCircle2 icon)
      expect(screen.getByText("a")).toBeInTheDocument();
      // Gate Passed should show since all have passed
      expect(screen.getByText("Gate Passed")).toBeInTheDocument();
    });

    it("treats done teammate as passed", () => {
      const teammates = [mate("a", "done")];
      render(<WaveGateIndicator currentWave={1} totalWaves={2} teammates={teammates} />);
      expect(screen.getByText("Gate Passed")).toBeInTheDocument();
    });

    it("treats teammate with wave > currentWave as passed", () => {
      const teammates = [mate("a", "active", 3)];
      render(<WaveGateIndicator currentWave={2} totalWaves={4} teammates={teammates} />);
      // a is on wave 3, current is 2 → passed
      expect(screen.getByText("Gate Passed")).toBeInTheDocument();
    });

    it("treats teammate with wave <= currentWave as working", () => {
      const teammates = [mate("a", "active", 2)];
      render(<WaveGateIndicator currentWave={2} totalWaves={3} teammates={teammates} />);
      // wave === currentWave → not passed, still working
      expect(screen.queryByText("Gate Passed")).not.toBeInTheDocument();
    });

    it("treats teammate with no wave and non-done status as working", () => {
      const teammates = [mate("a", "active")];
      render(<WaveGateIndicator currentWave={1} totalWaves={2} teammates={teammates} />);
      expect(screen.queryByText("Gate Passed")).not.toBeInTheDocument();
    });
  });

  describe("wave dots", () => {
    it("renders correct number of wave dots", () => {
      const { container } = render(
        <WaveGateIndicator
          currentWave={2}
          totalWaves={4}
          teammates={[mate("a", "active", 2)]}
        />
      );
      const dots = container.querySelectorAll(".w-1\\.5.h-1\\.5.rounded-full.transition-colors");
      expect(dots).toHaveLength(4);
    });

    it("colors completed waves with warm orange", () => {
      const { container } = render(
        <WaveGateIndicator
          currentWave={3}
          totalWaves={4}
          teammates={[mate("a", "active", 3)]}
        />
      );
      const dots = container.querySelectorAll(".w-1\\.5.h-1\\.5.rounded-full.transition-colors");
      // Wave 1 and 2 are completed (< currentWave=3) — design token
      expect(dots[0]).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
      expect(dots[1]).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });

    it("colors current wave with semi-transparent orange", () => {
      const { container } = render(
        <WaveGateIndicator
          currentWave={2}
          totalWaves={3}
          teammates={[mate("a", "active", 2)]}
        />
      );
      const dots = container.querySelectorAll(".w-1\\.5.h-1\\.5.rounded-full.transition-colors");
      // Wave 2 is current (index 1) — design token
      expect(dots[1]).toHaveStyle({ backgroundColor: "var(--accent-strong)" });
    });

    it("colors future waves with dim grey", () => {
      const { container } = render(
        <WaveGateIndicator
          currentWave={1}
          totalWaves={3}
          teammates={[mate("a", "active", 1)]}
        />
      );
      const dots = container.querySelectorAll(".w-1\\.5.h-1\\.5.rounded-full.transition-colors");
      // Waves 2 and 3 are future (index 1, 2) — design token
      expect(dots[1]).toHaveStyle({ backgroundColor: "var(--overlay-moderate)" });
      expect(dots[2]).toHaveStyle({ backgroundColor: "var(--overlay-moderate)" });
    });
  });

  describe("Gate Passed badge", () => {
    it("shows badge when all teammates passed", () => {
      const teammates = [
        mate("a", "completed"),
        mate("b", "done"),
      ];
      render(<WaveGateIndicator currentWave={1} totalWaves={2} teammates={teammates} />);
      expect(screen.getByText("Gate Passed")).toBeInTheDocument();
    });

    it("hides badge when some teammates still working", () => {
      const teammates = [
        mate("a", "completed"),
        mate("b", "active", 1),
      ];
      render(<WaveGateIndicator currentWave={1} totalWaves={2} teammates={teammates} />);
      expect(screen.queryByText("Gate Passed")).not.toBeInTheDocument();
    });
  });

  describe("wave header", () => {
    it("displays wave progress label", () => {
      render(
        <WaveGateIndicator
          currentWave={2}
          totalWaves={5}
          teammates={[mate("a", "active")]}
        />
      );
      expect(screen.getByText("Wave 2/5")).toBeInTheDocument();
    });
  });
});
