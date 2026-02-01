// TypeScript types for diff API (camelCase)

export type FileChangeStatus = "added" | "modified" | "deleted";

export interface FileChange {
  path: string;
  status: FileChangeStatus;
  additions: number;
  deletions: number;
}

export interface FileDiff {
  filePath: string;
  oldContent: string;
  newContent: string;
  language: string;
}
