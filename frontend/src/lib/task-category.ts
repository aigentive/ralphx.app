const TASK_CATEGORY_LABELS: Record<string, string> = {
  plan_merge: "Plan merge",
};

export function getTaskCategoryLabel(category: string | null | undefined): string {
  if (!category) return "";
  return TASK_CATEGORY_LABELS[category] ?? category;
}
