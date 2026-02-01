/**
 * Visual Regression Tests: ScreenshotGallery
 *
 * Professional screenshot gallery with lightbox and comparison mode
 */

import { test, expect } from "@playwright/test";
import { ScreenshotGalleryPage } from "../../../pages/components/screenshot-gallery.page";
import { openScreenshotGalleryTestPage } from "../../../helpers/screenshot-gallery.helpers";

test.describe("ScreenshotGallery", () => {
  let galleryPage: ScreenshotGalleryPage;

  test.beforeEach(async ({ page }) => {
    galleryPage = new ScreenshotGalleryPage(page);
  });

  test("empty state - displays when no screenshots", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "empty");

    // Verify empty state
    expect(await galleryPage.isEmpty()).toBe(true);
    await expect(galleryPage.emptyStateIcon).toBeVisible();
    await expect(galleryPage.emptyStateMessage).toHaveText("No screenshots captured");

    // Visual snapshot
    await expect(page).toHaveScreenshot("empty-state.png");
  });

  test("thumbnail grid - displays screenshots in grid layout", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    // Verify thumbnails
    expect(await galleryPage.getThumbnailCount()).toBe(4);
    await expect(galleryPage.getThumbnail(0)).toBeVisible();
    await expect(galleryPage.getThumbnail(3)).toBeVisible();

    // Visual snapshot
    await expect(page).toHaveScreenshot("thumbnail-grid.png");
  });

  test("thumbnail indicators - shows passed/failed/comparison badges", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    // Test data has: passed at index 1, failed at index 2 (with comparison)
    await expect(galleryPage.getPassedIndicator(1)).toBeVisible();
    await expect(galleryPage.getFailedIndicator(2)).toBeVisible();
    await expect(galleryPage.getComparisonIndicator(2)).toBeVisible();

    // Visual snapshot
    await expect(page).toHaveScreenshot("thumbnail-indicators.png");
  });

  test("lightbox - opens and displays full screenshot", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    // Open lightbox
    await galleryPage.openLightbox(0);

    // Verify lightbox
    expect(await galleryPage.isLightboxVisible()).toBe(true);
    await expect(galleryPage.lightboxFilename).toHaveText("step-1-login");
    await expect(galleryPage.lightboxCounter).toHaveText("1 / 4");

    // Visual snapshot
    await expect(page).toHaveScreenshot("lightbox-open.png");
  });

  test("lightbox navigation - prev/next buttons work", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    await galleryPage.openLightbox(0);

    // Navigate next
    await galleryPage.navigateNext();
    await expect(galleryPage.lightboxCounter).toHaveText("2 / 4");

    // Visual snapshot
    await expect(page).toHaveScreenshot("lightbox-navigation-next.png");

    // Navigate previous
    await galleryPage.navigatePrev();
    await expect(galleryPage.lightboxCounter).toHaveText("1 / 4");
  });

  test("lightbox keyboard navigation - arrow keys work", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    await galleryPage.openLightbox(1);

    // Arrow right
    await galleryPage.pressKey("ArrowRight");
    await expect(galleryPage.lightboxCounter).toHaveText("3 / 4");

    // Arrow left
    await galleryPage.pressKey("ArrowLeft");
    await expect(galleryPage.lightboxCounter).toHaveText("2 / 4");

    // Visual snapshot
    await expect(page).toHaveScreenshot("lightbox-keyboard-nav.png");
  });

  test("lightbox close - escape key closes lightbox", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    await galleryPage.openLightbox(0);
    expect(await galleryPage.isLightboxVisible()).toBe(true);

    // Press Escape
    await galleryPage.pressKey("Escape");
    expect(await galleryPage.isLightboxVisible()).toBe(false);
  });

  test("lightbox zoom - zoom in/out buttons work", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    await galleryPage.openLightbox(0);

    // Zoom in
    await galleryPage.zoomIn();
    await galleryPage.zoomIn();

    // Visual snapshot at zoom
    await expect(page).toHaveScreenshot("lightbox-zoomed-in.png");

    // Zoom out
    await galleryPage.zoomOut();

    await expect(page).toHaveScreenshot("lightbox-zoomed-out.png");
  });

  test("comparison mode - displays expected vs actual side-by-side", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    // Open failed screenshot with comparison (index 2)
    await galleryPage.openLightbox(2);

    // Toggle comparison
    await galleryPage.toggleComparison();
    expect(await galleryPage.isComparisonVisible()).toBe(true);

    // Verify both images visible
    await expect(galleryPage.comparisonExpectedImage).toBeVisible();
    await expect(galleryPage.comparisonActualImage).toBeVisible();

    // Visual snapshot
    await expect(page).toHaveScreenshot("comparison-mode.png");
  });

  test("comparison mode - keyboard shortcut 'c' toggles", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    await galleryPage.openLightbox(2);

    // Press 'c' to toggle comparison
    await galleryPage.pressKey("c");
    expect(await galleryPage.isComparisonVisible()).toBe(true);

    // Press 'c' again to toggle off
    await galleryPage.pressKey("c");
    expect(await galleryPage.isComparisonVisible()).toBe(false);
  });

  test("failure details - shows error message for failed screenshots", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    // Open failed screenshot (index 2)
    await galleryPage.openLightbox(2);

    // Verify failure details visible
    await expect(galleryPage.lightboxFailureDetails).toBeVisible();

    // Visual snapshot
    await expect(page).toHaveScreenshot("failure-details.png");
  });

  test("thumbnail strip - shows mini thumbnails in lightbox", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "default");

    await galleryPage.openLightbox(1);

    // Verify thumbnail strip
    await expect(galleryPage.getLightboxThumbnail(0)).toBeVisible();
    await expect(galleryPage.getLightboxThumbnail(3)).toBeVisible();

    // Click thumbnail to navigate
    await galleryPage.getLightboxThumbnail(3).click();
    await expect(galleryPage.lightboxCounter).toHaveText("4 / 4");

    // Visual snapshot
    await expect(page).toHaveScreenshot("lightbox-thumbnail-strip.png");
  });

  test("grid columns - respects column prop (2 columns)", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "twoColumns");

    // Visual snapshot
    await expect(page).toHaveScreenshot("grid-2-columns.png");
  });

  test("grid columns - respects column prop (4 columns)", async ({ page }) => {
    await openScreenshotGalleryTestPage(page, "fourColumns");

    // Visual snapshot
    await expect(page).toHaveScreenshot("grid-4-columns.png");
  });
});
