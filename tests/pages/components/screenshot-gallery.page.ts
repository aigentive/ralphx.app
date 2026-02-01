/**
 * Page Object: ScreenshotGallery
 *
 * Professional gallery with lightbox and comparison mode
 */

import { Locator, Page } from "@playwright/test";
import { BasePage } from "../base.page";

export class ScreenshotGalleryPage extends BasePage {
  // Gallery selectors
  readonly gallery: Locator;
  readonly emptyState: Locator;
  readonly emptyStateIcon: Locator;
  readonly emptyStateMessage: Locator;
  readonly thumbnailGrid: Locator;

  // Lightbox selectors
  readonly lightbox: Locator;
  readonly lightboxFilename: Locator;
  readonly lightboxCounter: Locator;
  readonly lightboxClose: Locator;
  readonly lightboxPrev: Locator;
  readonly lightboxNext: Locator;
  readonly lightboxZoomIn: Locator;
  readonly lightboxZoomOut: Locator;
  readonly lightboxToggleComparison: Locator;
  readonly comparisonView: Locator;
  readonly comparisonExpectedImage: Locator;
  readonly comparisonActualImage: Locator;
  readonly lightboxFailureDetails: Locator;

  constructor(page: Page) {
    super(page);

    // Gallery
    this.gallery = page.getByTestId("screenshot-gallery");
    this.emptyState = page.getByTestId("screenshot-gallery-empty");
    this.emptyStateIcon = this.emptyState.locator("svg").first();
    this.emptyStateMessage = this.emptyState.locator("p.text-sm").first();
    this.thumbnailGrid = this.gallery.locator("div.grid");

    // Lightbox
    this.lightbox = page.getByTestId("screenshot-lightbox");
    this.lightboxFilename = page.getByTestId("lightbox-filename");
    this.lightboxCounter = page.getByTestId("lightbox-counter");
    this.lightboxClose = page.getByTestId("lightbox-close");
    this.lightboxPrev = page.getByTestId("lightbox-prev");
    this.lightboxNext = page.getByTestId("lightbox-next");
    this.lightboxZoomIn = page.getByTestId("lightbox-zoom-in");
    this.lightboxZoomOut = page.getByTestId("lightbox-zoom-out");
    this.lightboxToggleComparison = page.getByTestId("lightbox-toggle-comparison");
    this.comparisonView = page.getByTestId("comparison-view");
    this.comparisonExpectedImage = page.getByTestId("comparison-expected-image");
    this.comparisonActualImage = page.getByTestId("comparison-actual-image");
    this.lightboxFailureDetails = page.getByTestId("lightbox-failure-details");
  }

  /**
   * Get thumbnail by index
   */
  getThumbnail(index: number): Locator {
    return this.page.getByTestId(`screenshot-thumbnail-${index}`);
  }

  /**
   * Get failed indicator for thumbnail
   */
  getFailedIndicator(index: number): Locator {
    return this.page.getByTestId(`screenshot-failed-indicator-${index}`);
  }

  /**
   * Get passed indicator for thumbnail
   */
  getPassedIndicator(index: number): Locator {
    return this.page.getByTestId(`screenshot-passed-indicator-${index}`);
  }

  /**
   * Get comparison indicator for thumbnail
   */
  getComparisonIndicator(index: number): Locator {
    return this.page.getByTestId(`screenshot-comparison-indicator-${index}`);
  }

  /**
   * Get lightbox thumbnail in strip
   */
  getLightboxThumbnail(index: number): Locator {
    return this.page.getByTestId(`lightbox-thumbnail-${index}`);
  }

  /**
   * Click thumbnail to open lightbox
   */
  async openLightbox(index: number): Promise<void> {
    await this.getThumbnail(index).click();
    await this.lightbox.waitFor({ state: "visible" });
  }

  /**
   * Close lightbox
   */
  async closeLightbox(): Promise<void> {
    await this.lightboxClose.click();
    await this.lightbox.waitFor({ state: "hidden" });
  }

  /**
   * Navigate to next screenshot in lightbox
   */
  async navigateNext(): Promise<void> {
    await this.lightboxNext.click();
    await this.page.waitForTimeout(200); // Animation
  }

  /**
   * Navigate to previous screenshot in lightbox
   */
  async navigatePrev(): Promise<void> {
    await this.lightboxPrev.click();
    await this.page.waitForTimeout(200); // Animation
  }

  /**
   * Zoom in
   */
  async zoomIn(): Promise<void> {
    await this.lightboxZoomIn.click();
    await this.page.waitForTimeout(100);
  }

  /**
   * Zoom out
   */
  async zoomOut(): Promise<void> {
    await this.lightboxZoomOut.click();
    await this.page.waitForTimeout(100);
  }

  /**
   * Toggle comparison mode
   */
  async toggleComparison(): Promise<void> {
    await this.lightboxToggleComparison.click();
    await this.page.waitForTimeout(200); // Animation
  }

  /**
   * Check if gallery is showing empty state
   */
  async isEmpty(): Promise<boolean> {
    return await this.emptyState.isVisible();
  }

  /**
   * Check if lightbox is visible
   */
  async isLightboxVisible(): Promise<boolean> {
    return await this.lightbox.isVisible();
  }

  /**
   * Check if comparison view is visible
   */
  async isComparisonVisible(): Promise<boolean> {
    return await this.comparisonView.isVisible();
  }

  /**
   * Get thumbnail count
   */
  async getThumbnailCount(): Promise<number> {
    return await this.thumbnailGrid.locator("button").count();
  }

  /**
   * Get lightbox filename text
   */
  async getLightboxFilename(): Promise<string> {
    return await this.lightboxFilename.textContent() || "";
  }

  /**
   * Get lightbox counter text
   */
  async getLightboxCounter(): Promise<string> {
    return await this.lightboxCounter.textContent() || "";
  }

  /**
   * Press keyboard key
   */
  async pressKey(key: string): Promise<void> {
    await this.page.keyboard.press(key);
    await this.page.waitForTimeout(100);
  }
}
