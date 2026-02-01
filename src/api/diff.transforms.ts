// Transform functions for diff API (snake_case -> camelCase)

import type { z } from "zod";
import type { FileChangeSchema, FileDiffSchema } from "./diff.schemas";
import type { FileChange, FileDiff } from "./diff.types";

type RawFileChange = z.infer<typeof FileChangeSchema>;
type RawFileDiff = z.infer<typeof FileDiffSchema>;

export function transformFileChange(raw: RawFileChange): FileChange {
  return {
    path: raw.path,
    status: raw.status,
    additions: raw.additions,
    deletions: raw.deletions,
  };
}

export function transformFileDiff(raw: RawFileDiff): FileDiff {
  return {
    filePath: raw.file_path,
    oldContent: raw.old_content,
    newContent: raw.new_content,
    language: raw.language,
  };
}
