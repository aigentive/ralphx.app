const IDENTIFIER_RE = /\b[A-Z][A-Z0-9]{1,9}-\d+\b/;
const ACTION_VERBS = new Set([
  "add",
  "analyze",
  "build",
  "create",
  "debug",
  "design",
  "document",
  "fix",
  "implement",
  "investigate",
  "plan",
  "refactor",
  "remove",
  "review",
  "test",
  "update",
  "write",
]);

export function isDefaultAgentTitle(title: string | null | undefined): boolean {
  const normalized = title?.trim().toLowerCase();
  return !normalized || normalized === "untitled agent";
}

export function deriveAgentTitleFromMessages(messages: string[]): string | null {
  const candidates = messages
    .map(cleanTitleSource)
    .filter((message) => message.length > 0);
  if (candidates.length === 0) {
    return null;
  }

  const source = candidates
    .slice(0, 3)
    .sort((a, b) => scoreSource(b) - scoreSource(a))[0];
  if (!source) {
    return null;
  }

  const identifier = source.match(IDENTIFIER_RE)?.[0] ?? null;
  const phrase = normalizeTitlePhrase(source);
  if (!phrase) {
    return null;
  }

  const title = identifier && !phrase.includes(identifier)
    ? `${identifier}: ${phrase}`
    : phrase;
  return truncateTitle(title);
}

function cleanTitleSource(value: string): string {
  return value
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/https?:\/\/\S+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

function scoreSource(value: string): number {
  const words = value.split(/\s+/).filter(Boolean).length;
  const hasAction = ACTION_VERBS.has(value.split(/\s+/)[0]?.toLowerCase() ?? "");
  return words + (hasAction ? 10 : 0) + (value.length >= 8 ? 4 : 0);
}

function normalizeTitlePhrase(source: string): string {
  let phrase = source
    .replace(IDENTIFIER_RE, " ")
    .replace(/\s+/g, " ")
    .trim()
    .replace(/^[,.;:!?]+|[,.;:!?]+$/g, "")
    .replace(/^please\s+/i, "")
    .replace(/^(can you|could you|would you)\s+/i, "")
    .replace(/^(i\s+)?(want|need|would like)\s+to\s+/i, "")
    .replace(/^how\s+(do|can|should)\s+i\s+/i, "")
    .replace(/^let'?s\s+/i, "")
    .replace(/^please\s+/i, "")
    .replace(/\s+/g, " ")
    .trim();

  if (!phrase) {
    return "";
  }

  const firstWord = phrase.split(/\s+/)[0]?.toLowerCase() ?? "";
  if (!ACTION_VERBS.has(firstWord)) {
    phrase = `Discuss ${phrase}`;
  }

  return sentenceCaseFirstWord(phrase);
}

function sentenceCaseFirstWord(value: string): string {
  const [first = "", ...rest] = value.split(/\s+/);
  if (!first) {
    return value;
  }
  return [first[0]?.toUpperCase() + first.slice(1), ...rest].join(" ");
}

function truncateTitle(value: string): string {
  const maxLength = 50;
  if (value.length <= maxLength) {
    return value;
  }

  const truncated = value.slice(0, maxLength).replace(/\s+\S*$/, "").trim();
  return truncated || value.slice(0, maxLength).trim();
}
