/**
 * ExtensibilityView Shared Utilities
 * Helpers and types shared across panel components
 */

import {
  FileText,
  FileJson,
  FileCode,
  Image,
  File,
  Zap,
  Target,
  Telescope,
  Microscope,
  Sliders,
} from "lucide-react";

// ============================================================================
// Types
// ============================================================================

export type ViewMode = "grid" | "list";
export type SortBy = "name" | "date" | "size" | "type";

// ============================================================================
// Helpers
// ============================================================================

/** Get file type icon based on artifact type or extension */
export function getFileIcon(type: string) {
  switch (type.toLowerCase()) {
    case "markdown":
    case "md":
      return FileText;
    case "json":
      return FileJson;
    case "code":
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
    case "rs":
      return FileCode;
    case "image":
    case "png":
    case "jpg":
    case "jpeg":
    case "svg":
      return Image;
    default:
      return File;
  }
}

/** Get depth preset icon */
export function getDepthIcon(preset: string) {
  switch (preset) {
    case "quick-scan":
      return Zap;
    case "standard":
      return Target;
    case "deep-dive":
      return Telescope;
    case "exhaustive":
      return Microscope;
    default:
      return Sliders;
  }
}
