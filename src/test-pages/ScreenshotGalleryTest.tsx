/**
 * Test Page: ScreenshotGallery
 * For visual regression testing in web mode
 */

import { ScreenshotGallery } from "@/components/qa/ScreenshotGallery/ScreenshotGallery";
import type { Screenshot } from "@/components/qa/ScreenshotGallery/ScreenshotGallery";

// Mock screenshot data for testing
const mockScreenshots: Screenshot[] = [
  {
    id: "screenshot-1",
    path: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='300'%3E%3Crect width='400' height='300' fill='%234a5568'/%3E%3Ctext x='50%25' y='50%25' fill='white' text-anchor='middle' font-size='20'%3EStep 1 - Login%3C/text%3E%3C/svg%3E",
    label: "step-1-login",
    timestamp: "2026-01-31T10:00:00Z",
  },
  {
    id: "screenshot-2",
    path: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='300'%3E%3Crect width='400' height='300' fill='%2348bb78'/%3E%3Ctext x='50%25' y='50%25' fill='white' text-anchor='middle' font-size='20'%3EStep 2 - Dashboard%3C/text%3E%3C/svg%3E",
    label: "step-2-dashboard",
    timestamp: "2026-01-31T10:01:00Z",
    stepResult: {
      step_id: "step-2",
      status: "passed",
      started_at: "2026-01-31T10:01:00Z",
      completed_at: "2026-01-31T10:01:05Z",
      duration_ms: 5000,
    },
  },
  {
    id: "screenshot-3",
    path: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='300'%3E%3Crect width='400' height='300' fill='%23e53e3e'/%3E%3Ctext x='50%25' y='50%25' fill='white' text-anchor='middle' font-size='20'%3EStep 3 - Form Error%3C/text%3E%3C/svg%3E",
    label: "step-3-form-error",
    timestamp: "2026-01-31T10:02:00Z",
    stepResult: {
      step_id: "step-3",
      status: "failed",
      error: "Form validation failed: Email is required",
      expected: "Form submitted successfully",
      actual: "Form validation error displayed",
      started_at: "2026-01-31T10:02:00Z",
      completed_at: "2026-01-31T10:02:03Z",
      duration_ms: 3000,
    },
    expectedPath: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='300'%3E%3Crect width='400' height='300' fill='%2348bb78'/%3E%3Ctext x='50%25' y='50%25' fill='white' text-anchor='middle' font-size='18'%3EExpected: Success Page%3C/text%3E%3C/svg%3E",
  },
  {
    id: "screenshot-4",
    path: "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='400' height='300'%3E%3Crect width='400' height='300' fill='%23667eea'/%3E%3Ctext x='50%25' y='50%25' fill='white' text-anchor='middle' font-size='20'%3EStep 4 - Settings%3C/text%3E%3C/svg%3E",
    label: "step-4-settings",
    timestamp: "2026-01-31T10:03:00Z",
  },
];

interface ScreenshotGalleryTestPageProps {
  screenshots?: Screenshot[];
  columns?: 2 | 3 | 4;
  emptyMessage?: string;
}

export function ScreenshotGalleryTestPage({
  screenshots = mockScreenshots,
  columns = 3,
  emptyMessage,
}: ScreenshotGalleryTestPageProps = {}) {
  return (
    <div style={{ padding: "2rem", maxWidth: "1200px", margin: "0 auto" }}>
      <h1 style={{ marginBottom: "1.5rem", fontSize: "1.5rem", fontWeight: 600 }}>
        Screenshot Gallery Test
      </h1>
      <ScreenshotGallery
        screenshots={screenshots}
        columns={columns}
        emptyMessage={emptyMessage}
      />
    </div>
  );
}

// Export scenarios for testing
export const scenarios = {
  default: <ScreenshotGalleryTestPage />,
  empty: <ScreenshotGalleryTestPage screenshots={[]} />,
  twoColumns: <ScreenshotGalleryTestPage columns={2} />,
  fourColumns: <ScreenshotGalleryTestPage columns={4} screenshots={Array.from({ length: 8 }, (_, i) => mockScreenshots[i % mockScreenshots.length]!)} />,
};
