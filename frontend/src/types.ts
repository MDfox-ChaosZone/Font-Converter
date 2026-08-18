export type ConversionKind =
  | "ttf_to_woff2"
  | "otf_to_woff2"
  | "woff2_to_ttf"
  | "woff2_to_otf";

export type FolderConversionMode = "font_to_woff2" | "woff2_to_font" | "both";

export type ItemStatus =
  | "queued"
  | "running"
  | "succeeded"
  | "skipped"
  | "failed"
  | "cancelled";

export type ErrorCode =
  | "input_not_found"
  | "input_unreadable"
  | "invalid_font"
  | "unsupported_format"
  | "output_exists"
  | "output_conflict"
  | "output_unwritable"
  | "input_too_large"
  | "conversion_failed"
  | "cancelled";

export interface QueueItem {
  id: string;
  conversion: ConversionKind;
  inputPath: string;
  outputPath: string;
  inputBytes: number | null;
  outputBytes: number | null;
  status: ItemStatus;
  errorCode: ErrorCode | null;
  message: string | null;
}

export interface ScanWarning {
  path: string;
  errorCode: ErrorCode;
  message: string;
}

export interface ScanResult {
  items: QueueItem[];
  warnings: ScanWarning[];
}

export interface BatchSummary {
  total: number;
  queued: number;
  running: number;
  succeeded: number;
  skipped: number;
  failed: number;
  cancelled: number;
}

export interface ProgressEvent {
  batchId: string;
  item: QueueItem | null;
  summary: BatchSummary;
  finished: boolean;
}
