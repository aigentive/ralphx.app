export function EmptyArtifactState({ title, detail }: { title: string; detail?: string | undefined }) {
  return (
    <div className="h-full min-h-[220px] flex items-center justify-center p-6 text-center">
      <div className="max-w-sm">
        <div className="text-sm font-medium text-[var(--text-primary)]">{title}</div>
        {detail && <div className="mt-2 text-xs leading-relaxed text-[var(--text-muted)]">{detail}</div>}
      </div>
    </div>
  );
}
