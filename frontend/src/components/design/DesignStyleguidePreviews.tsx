import { Loader2, Package, Sparkles } from "lucide-react";

import type { DesignStyleguidePreviewResponse } from "@/api/design";
import type { DesignStyleguideItem } from "./designSystems";
import { useDesignStyleguidePreview } from "./useProjectDesignSystems";

type DesignPreviewContent = DesignStyleguidePreviewResponse["content"];

export function PreviewBlock({ designSystemId, item }: { designSystemId: string; item: DesignStyleguideItem }) {
  const persistedPreviewArtifactId = item.isPersisted ? item.previewArtifactId ?? null : null;
  const previewQuery = useDesignStyleguidePreview(
    designSystemId,
    persistedPreviewArtifactId,
  );
  const preview = previewQuery.data?.content;
  const sourceCount = preview?.source_refs.length ?? item.sourceRefs.length;
  const previewKind = preview?.preview_kind ?? previewKindForItem(item);

  if (!item.isPersisted) {
    return (
      <PreviewSample
        item={item}
        preview={undefined}
        previewKind={previewKind}
        sourceCount={sourceCount}
      />
    );
  }

  if (!item.previewArtifactId) {
    return (
      <div
        className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
        style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
        data-testid="design-preview-empty"
      >
        <Package className="w-4 h-4" style={{ color: "var(--text-muted)" }} />
        <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
          Preview pending
        </span>
      </div>
    );
  }

  if (previewQuery.isLoading) {
    return (
      <div
        className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
        style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
        data-testid="design-preview-loading"
      >
        <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--accent-primary)" }} />
        <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
          Loading preview
        </span>
      </div>
    );
  }

  return (
    <PreviewSample
      item={item}
      preview={preview}
      previewKind={previewKind}
      sourceCount={sourceCount}
    />
  );
}

function PreviewSample({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  if (previewKind === "color_swatch") {
    return <ColorPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "typography_sample") {
    return <TypographyPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "spacing_sample") {
    return <SpacingPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "layout_sample" || previewKind === "screen_artifact_preview") {
    return <LayoutPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "asset_sample") {
    return <AssetPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  return <ComponentPreview item={item} preview={preview} previewKind={previewKind} sourceCount={sourceCount} />;
}

type PreviewRecord = Record<string, unknown>;

function previewRecords(preview: DesignPreviewContent | undefined, key: string): PreviewRecord[] {
  const value = (preview as PreviewRecord | undefined)?.[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is PreviewRecord => Boolean(entry) && typeof entry === "object" && !Array.isArray(entry));
}

function previewStringArray(preview: DesignPreviewContent | undefined, key: string): string[] {
  const value = (preview as PreviewRecord | undefined)?.[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is string => typeof entry === "string" && entry.trim().length > 0);
}

function recordString(record: PreviewRecord, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function sourceLabelsForPreview(
  item: DesignStyleguideItem,
  preview: DesignPreviewContent | undefined,
  fallbackCount = 4,
): string[] {
  const labels = previewStringArray(preview, "source_labels");
  if (labels.length > 0) {
    return labels.slice(0, fallbackCount);
  }
  const paths = previewStringArray(preview, "source_paths");
  const pathLabels = paths.map(labelFromSourcePath).filter(Boolean);
  if (pathLabels.length > 0) {
    return pathLabels.slice(0, fallbackCount);
  }
  const itemLabels = item.sourceRefs.map((sourceRef) => labelFromSourcePath(sourceRef.path));
  return itemLabels.length > 0 ? itemLabels.slice(0, fallbackCount) : [item.label];
}

function labelFromSourcePath(path: string): string {
  const filename = path.split("/").pop() ?? path;
  const stem = filename.replace(/\.[^.]+$/, "");
  return stem
    .replace(/[-_.]+/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (match) => match.toUpperCase())
    .trim();
}

function SourceBackedPreview({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  const labels = sourceLabelsForPreview(item, preview, 6);
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-source-backed-preview"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {labels.map((label) => (
          <div
            key={label}
            className="rounded-md border px-2.5 py-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div className="text-[11px] font-medium" style={{ color: "var(--text-primary)" }}>
              {label}
            </div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function ColorPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const swatches = previewRecords(preview, "swatches")
    .map((swatch, index) => ({
      label: recordString(swatch, "label") ?? `Source ${index + 1}`,
      value: recordString(swatch, "value"),
    }))
    .filter((swatch): swatch is { label: string; value: string } => Boolean(swatch.value));

  if (swatches.length === 0) {
    return (
      <SourceBackedPreview
        item={item}
        preview={preview}
        previewKind={preview?.preview_kind ?? "color_swatch"}
        sourceCount={sourceCount}
      />
    );
  }

  return (
    <div className="space-y-2" data-testid="design-color-preview">
      <div className="grid grid-cols-2 gap-2">
        {swatches.map((swatch) => (
          <div
            key={swatch.label}
            className="min-h-20 rounded-lg border p-2"
            style={{
              borderColor: "var(--overlay-weak)",
              background: swatch.value,
              color: readableTextColor(swatch.value),
            }}
          >
            <div className="text-[10px] font-semibold uppercase">{swatch.label}</div>
            <div className="mt-5 text-[11px]">{swatch.value}</div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "color_swatch"} sourceCount={sourceCount} />
    </div>
  );
}

function readableTextColor(color: string): string {
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    const normalized = hex.length === 3
      ? hex.split("").map((value) => `${value}${value}`).join("")
      : hex.slice(0, 6);
    const red = Number.parseInt(normalized.slice(0, 2), 16);
    const green = Number.parseInt(normalized.slice(2, 4), 16);
    const blue = Number.parseInt(normalized.slice(4, 6), 16);
    if ([red, green, blue].every(Number.isFinite)) {
      return red * 0.299 + green * 0.587 + blue * 0.114 > 150 ? "#111111" : "#ffffff";
    }
  }
  return "var(--text-primary)";
}

function TypographyPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const samples = previewRecords(preview, "typography_samples");
  const rows = samples.length
    ? samples.map((sample, index) => ({
        label: recordString(sample, "label") ?? ["Display", "Body", "Label", "Code"][index] ?? "Type",
        sample: recordString(sample, "sample") ?? item.label,
        className: [
          "text-[20px] leading-7 font-semibold",
          "text-[13px] leading-5",
          "text-[11px] leading-4 font-semibold uppercase",
          "font-mono text-[12px] leading-5",
        ][index] ?? "text-[13px] leading-5",
      }))
    : [
        { label: "Display", sample: preview?.label ?? item.label, className: "text-[20px] leading-7 font-semibold" },
        { label: "Body", sample: preview?.summary ?? item.summary, className: "text-[13px] leading-5" },
        { label: "Label", sample: item.confidence.toUpperCase(), className: "text-[11px] leading-4 font-semibold uppercase" },
        { label: "Code", sample: item.sourceRefs[0]?.path ?? item.itemId, className: "font-mono text-[12px] leading-5" },
      ];
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-typography-preview"
    >
      <div className="space-y-1">
        <div className="text-[28px] leading-9 font-semibold" style={{ color: "var(--text-primary)" }}>
          {preview?.label ?? item.label}
        </div>
        <div className="text-[13px] leading-5" style={{ color: "var(--text-secondary)" }}>
          {preview?.summary ?? item.summary}
        </div>
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        {rows.map((row) => (
          <div
            key={row.label}
            className="rounded-md border px-2.5 py-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
              {row.label}
            </div>
            <div className={row.className} style={{ color: "var(--text-primary)" }}>
              {row.sample}
            </div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "typography_sample"} sourceCount={sourceCount} />
    </div>
  );
}

function ComponentPreview({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  const componentSamples = previewRecords(preview, "component_samples")
    .map((sample) => recordString(sample, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = componentSamples.length > 0
    ? componentSamples
    : sourceLabelsForPreview(item, preview, 4);
  const stateLabels = ["default", "hover", "focus", "loading"];

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-component-preview"
    >
      <div className="flex flex-wrap items-center gap-2">
        {labels.map((label, index) => (
          <button
            key={`${label}-${index}`}
            type="button"
            className="h-8 rounded-md border px-3 text-[12px] font-medium"
            style={{
              borderColor: index === 0 ? "var(--accent-border)" : "var(--overlay-weak)",
              color: index === 0 ? "var(--accent-primary)" : "var(--text-primary)",
              background: index === 0 ? "var(--accent-muted)" : "var(--bg-base)",
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <div
        className="rounded-md border p-2"
        style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
      >
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {stateLabels.map((stateLabel) => (
            <div
              key={stateLabel}
              className="rounded-md border px-2 py-1.5 text-[11px]"
              style={{ borderColor: "var(--overlay-weak)", color: "var(--text-secondary)", background: "var(--bg-surface)" }}
            >
              {stateLabel}
            </div>
          ))}
        </div>
      </div>
      <div className="flex items-center justify-between gap-3 text-[12px]">
        <div className="min-w-0">
          <div className="font-medium truncate" style={{ color: "var(--text-primary)" }}>
            {preview?.label ?? item.label}
          </div>
          <div className="truncate" style={{ color: "var(--text-secondary)" }}>
            {preview?.summary ?? item.summary}
          </div>
        </div>
        <span
          className="shrink-0 rounded-full border px-2 py-1 text-[11px]"
          style={{ borderColor: "var(--overlay-faint)", color: "var(--text-secondary)" }}
        >
          {item.confidence} confidence
        </span>
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
    </div>
  );
}

function SpacingPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const regions = previewRecords(preview, "layout_regions")
    .map((region) => recordString(region, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = regions.length > 0 ? regions.slice(0, 3) : sourceLabelsForPreview(item, preview, 3);

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-spacing-preview"
    >
      <div className="grid grid-cols-4 gap-2">
        {[4, 8, 12, 16].map((size) => (
          <div
            key={size}
            className="rounded-md border p-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div
              className="rounded-sm"
              style={{
                height: `${Math.max(size, 8)}px`,
                background: "var(--accent-muted)",
              }}
            />
            <div className="mt-2 text-[11px]" style={{ color: "var(--text-secondary)" }}>
              {size}px
            </div>
          </div>
        ))}
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        <div className="h-14 rounded-lg border" style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-base)" }} />
        <div className="h-14 rounded-lg border shadow-lg" style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }} />
      </div>
      <div className="flex flex-wrap gap-2">
        {labels.map((label) => (
          <span
            key={label}
            className="rounded-md border px-2 py-1 text-[11px]"
            style={{
              borderColor: "var(--overlay-faint)",
              background: "var(--bg-base)",
              color: "var(--text-secondary)",
            }}
          >
            {label}
          </span>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "spacing_sample"} sourceCount={sourceCount} />
      <div className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function LayoutPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const regions = previewRecords(preview, "layout_regions")
    .map((region) => recordString(region, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = regions.length > 0 ? regions.slice(0, 3) : sourceLabelsForPreview(item, preview, 3);

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-layout-preview"
    >
      <div
        className="grid h-36 overflow-hidden rounded-md border"
        style={{
          borderColor: "var(--overlay-faint)",
          gridTemplateColumns: labels.length >= 3 ? "0.8fr 1.4fr 1fr" : "1fr 1fr",
        }}
        data-testid="design-workspace-surface-preview"
      >
        {labels.map((label, index) => (
          <div
            key={`${label}-${index}`}
            className={index < labels.length - 1 ? "border-r p-2 space-y-2" : "p-2 space-y-2"}
            style={{ borderColor: "var(--overlay-faint)", background: index % 2 === 0 ? "var(--bg-base)" : "transparent" }}
          >
            <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
              {label}
            </div>
            <div className="h-3 w-16 rounded-sm" style={{ background: "var(--overlay-weak)" }} />
            <div className="h-6 rounded-md" style={{ background: index === 0 ? "var(--accent-muted)" : "var(--overlay-faint)" }} />
            <div className="h-6 rounded-md border" style={{ borderColor: "var(--overlay-faint)" }} />
          </div>
        ))}
      </div>
      <div className="text-[12px] font-medium" style={{ color: "var(--text-primary)" }}>
        {preview?.label ?? item.label}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "layout_sample"} sourceCount={sourceCount} />
      <div className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function AssetPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const assets = previewRecords(preview, "asset_samples")
    .map((asset) => ({
      label: recordString(asset, "label"),
      path: recordString(asset, "path"),
    }))
    .filter((asset): asset is { label: string; path: string | null } => Boolean(asset.label));
  const labels = assets.length > 0
    ? assets
    : sourceLabelsForPreview(item, preview, 4).map((label) => ({ label, path: null }));

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-asset-preview"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {labels.map((asset) => (
          <div key={asset.label} className="flex items-center gap-3 rounded-md border p-2" style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}>
            <div
              className="flex h-10 w-10 items-center justify-center rounded-lg border"
              style={{ borderColor: "var(--accent-border)", background: "var(--accent-muted)", color: "var(--accent-primary)" }}
            >
              <Sparkles className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <div className="truncate text-[12px] font-semibold" style={{ color: "var(--text-primary)" }}>
                {asset.label}
              </div>
              {asset.path ? (
                <div className="truncate text-[11px]" style={{ color: "var(--text-muted)" }}>
                  {asset.path}
                </div>
              ) : null}
            </div>
          </div>
        ))}
      </div>
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "asset_sample"} sourceCount={sourceCount} />
    </div>
  );
}

function PreviewMeta({ previewKind, sourceCount }: { previewKind: string; sourceCount: number }) {
  return (
    <div className="text-[11px]" style={{ color: "var(--text-muted)" }} data-testid="design-preview-kind">
      {formatPreviewKind(previewKind)} / {sourceCount} {sourceCount === 1 ? "source" : "sources"}
    </div>
  );
}

function previewKindForItem(item: DesignStyleguideItem): string {
  switch (item.group) {
    case "colors":
      return "color_swatch";
    case "type":
      return "typography_sample";
    case "spacing":
      return "spacing_sample";
    case "ui_kit":
      return "layout_sample";
    case "brand":
      return "asset_sample";
    case "components":
    default:
      return "component_sample";
  }
}

function formatPreviewKind(previewKind: string): string {
  return previewKind.replace(/_/g, " ");
}
