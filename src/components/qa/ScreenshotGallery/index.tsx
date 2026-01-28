export {
  ScreenshotGallery,
  type Screenshot,
  type ScreenshotGalleryProps,
} from "./ScreenshotGallery";
// Note: pathsToScreenshots is available from "./ScreenshotGallery/utils"
// Not re-exported here to avoid react-refresh lint warning (mixing components with utilities)

export { ScreenshotGallery as default } from "./ScreenshotGallery";
