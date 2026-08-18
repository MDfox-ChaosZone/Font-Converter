import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type {
  FolderConversionMode,
  ProgressEvent,
  QueueItem,
  ScanResult,
} from "./types";

export function pickFiles(): Promise<string[]> {
  requireTauri();
  return invoke("pick_files");
}

export function pickFolder(): Promise<string[]> {
  requireTauri();
  return invoke("pick_folder");
}

export function collectInputs(
  paths: string[],
  outputDirectory: string | null,
  folderConversionMode: FolderConversionMode | null,
): Promise<ScanResult> {
  requireTauri();
  return invoke("collect_inputs", { paths, outputDirectory, folderConversionMode });
}

export function startConversion(items: QueueItem[], parallelism: number): Promise<string> {
  requireTauri();
  return invoke("start_conversion", { items, parallelism });
}

export function cancelConversion(batchId: string): Promise<boolean> {
  requireTauri();
  return invoke("cancel_conversion", { batchId });
}

export function openOutputFolder(outputPath: string): Promise<void> {
  requireTauri();
  return invoke("open_output_folder", { outputPath });
}

export function listenProgress(callback: (event: ProgressEvent) => void): Promise<UnlistenFn> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen<ProgressEvent>("conversion-progress", (event) => callback(event.payload));
}

export async function listenDragDrop(
  callback: (isDragging: boolean, paths: string[]) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) return () => undefined;
  return getCurrentWebview().onDragDropEvent((event) => {
    const payload = event.payload;
    if (payload.type === "drop") {
      callback(false, payload.paths);
    } else if (payload.type === "enter" || payload.type === "over") {
      callback(true, []);
    } else {
      callback(false, []);
    }
  });
}

function requireTauri(): void {
  if (!isTauri()) throw new Error("Tauri is unavailable in this browser preview");
}
