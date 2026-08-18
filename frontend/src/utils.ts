import type {
  BatchSummary,
  ConversionKind,
  FolderConversionMode,
  ItemStatus,
  QueueItem,
} from "./types";
import type { Locale, MessageKey } from "./i18n";

export function summarize(items: QueueItem[]): BatchSummary {
  const summary: BatchSummary = {
    total: items.length,
    queued: 0,
    running: 0,
    succeeded: 0,
    skipped: 0,
    failed: 0,
    cancelled: 0,
  };
  for (const item of items) summary[item.status] += 1;
  return summary;
}

export function isFinished(status: ItemStatus): boolean {
  return status === "succeeded" || status === "skipped" || status === "failed" || status === "cancelled";
}

export function accepts(mode: FolderConversionMode, conversion: ConversionKind): boolean {
  if (mode === "both") return true;
  if (mode === "font_to_woff2") return conversion === "ttf_to_woff2" || conversion === "otf_to_woff2";
  return conversion === "woff2_to_ttf" || conversion === "woff2_to_otf";
}

export function conversionBadge(conversion: ConversionKind): string {
  return {
    ttf_to_woff2: "TTF→WOFF2",
    otf_to_woff2: "OTF→WOFF2",
    woff2_to_ttf: "WOFF2→TTF",
    woff2_to_otf: "WOFF2→OTF",
  }[conversion];
}

export function statusKey(status: ItemStatus): MessageKey {
  return status;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}

export function displayPath(path: string): string {
  if (path.startsWith("\\\\?\\UNC\\")) return `\\\\${path.slice(8)}`;
  return path.startsWith("\\\\?\\") ? path.slice(4) : path;
}

export function formatBytes(bytes: number | null): string {
  if (bytes === null) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatSizeChange(input: number | null, output: number | null): string {
  if (input === null || output === null || input === 0) return "—";
  const change = ((output - input) / input) * 100;
  if (Math.abs(change) < 0.05) return "0.0%";
  return `${change >= 0 ? "+" : ""}${change.toFixed(1)}%`;
}

export function localizedMessage(locale: Locale, message: string): string {
  if (locale === "en") return message;
  return {
    "Output file already exists": "输出文件已存在",
    "Output path conflicts with another queued font": "输出路径与队列中的另一字体冲突",
    "Cancelled before conversion started": "转换开始前已取消",
    "Invalid or changed input font": "输入字体无效或已发生变化",
    "Invalid or changed conversion path": "转换路径无效或已发生变化",
    "Input does not contain a valid WOFF2 header": "输入文件不包含有效的 WOFF2 文件头",
    "The WOFF2 output size is invalid or exceeds the 128 MB safety limit": "WOFF2 解压后大小无效或超过 128 MB 安全限制",
    "Google WOFF2 rejected the file or failed to decompress it": "Google WOFF2 拒绝该文件或解压失败",
    "Input font is empty": "输入字体为空",
    "Input font exceeds the 256 MB safety limit": "输入字体超过 256 MB 安全限制",
    "Google WOFF2 could not determine a safe output size": "Google WOFF2 无法确定安全的输出大小",
    "Google WOFF2 rejected the font or failed to encode it": "Google WOFF2 拒绝该字体或编码失败",
  }[message] ?? message;
}
