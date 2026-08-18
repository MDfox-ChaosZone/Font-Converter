<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import * as api from "./api";
import { loadLocale, loadTheme, translate, type Locale, type MessageKey, type Theme } from "./i18n";
import type { FolderConversionMode, ProgressEvent, QueueItem, ScanResult, ScanWarning } from "./types";
import {
  accepts,
  conversionBadge,
  displayPath,
  fileName,
  formatBytes,
  formatSizeChange,
  isFinished,
  localizedMessage,
  statusKey,
  summarize,
} from "./utils";

const MIN_COLUMN_WIDTHS = [68, 120, 140, 88, 56] as const;
const DEFAULT_PARALLELISM = 4;
const MAX_PARALLELISM = 32;

const locale = ref<Locale>(loadLocale());
const theme = ref<Theme>(loadTheme());
const items = ref<QueueItem[]>([]);
const warnings = ref<ScanWarning[]>([]);
const activeBatch = ref<string | null>(null);
const scanning = ref(false);
const dragging = ref(false);
const outputDirectory = ref<string | null>(null);
const pendingFolder = ref<string | null>(null);
const pendingFolderScan = ref<ScanResult | null>(null);
const folderConversionMode = ref<FolderConversionMode>("both");
const error = ref<string | null>(null);
const columnWidths = ref([78, 160, 180, 96, 62]);
const resizing = ref<{ index: number; startX: number; startWidth: number } | null>(null);
const parallelism = ref(loadParallelism());
const unlisteners: Array<() => void> = [];

const t = (key: MessageKey) => translate(locale.value, key);
const summary = computed(() => summarize(items.value));
const pendingCount = computed(() => summary.value.queued);
const finishedCount = computed(
  () => summary.value.succeeded + summary.value.skipped + summary.value.failed + summary.value.cancelled,
);
const progressPercent = computed(() =>
  summary.value.total === 0 ? 0 : Math.floor((finishedCount.value * 100) / summary.value.total),
);
const queueColumnsStyle = computed(() => ({
  "--direction-column": `${columnWidths.value[0]}px`,
  "--name-column": `${columnWidths.value[1]}px`,
  "--path-column": `${columnWidths.value[2]}px`,
  "--size-column": `${columnWidths.value[3]}px`,
  "--status-column": `${columnWidths.value[4]}px`,
}));

watch(locale, (value) => {
  localStorage.setItem("font-converter.locale", value);
  document.documentElement.lang = value;
});
watch(theme, (value) => {
  localStorage.setItem("font-converter.theme", value);
  document.documentElement.dataset.theme = value;
});
watch(parallelism, (value) => localStorage.setItem("font-converter.parallelism", String(value)));

onMounted(async () => {
  document.documentElement.lang = locale.value;
  document.documentElement.dataset.theme = theme.value;
  window.addEventListener("pointermove", resizeColumn);
  window.addEventListener("pointerup", stopResize);
  try {
    unlisteners.push(await api.listenProgress(handleProgress));
    unlisteners.push(await api.listenDragDrop(handleDragDrop));
  } catch (cause) {
    error.value = errorMessage(cause);
  }
});

onUnmounted(() => {
  window.removeEventListener("pointermove", resizeColumn);
  window.removeEventListener("pointerup", stopResize);
  for (const unlisten of unlisteners) unlisten();
});

function loadParallelism(): number {
  const value = Number(localStorage.getItem("font-converter.parallelism"));
  return Number.isInteger(value) && value >= 1 && value <= MAX_PARALLELISM ? value : DEFAULT_PARALLELISM;
}

function updateParallelism(event: Event) {
  const value = Number((event.target as HTMLInputElement).value);
  parallelism.value = Number.isInteger(value) ? Math.min(MAX_PARALLELISM, Math.max(1, value)) : DEFAULT_PARALLELISM;
}

function handleDragDrop(isDragging: boolean, paths: string[]) {
  dragging.value = isDragging;
  if (paths.length > 0) void addPaths(paths);
}

function handleProgress(event: ProgressEvent) {
  if (activeBatch.value === "") activeBatch.value = event.batchId;
  if (event.item) {
    const index = items.value.findIndex((item) => item.id === event.item?.id);
    if (index >= 0) items.value[index] = event.item;
  }
  const matchesActive = activeBatch.value === "" || activeBatch.value === event.batchId;
  if (event.finished && matchesActive) activeBatch.value = null;
}

async function addPaths(paths: string[]) {
  if (paths.length === 0 || scanning.value || activeBatch.value !== null) return;
  scanning.value = true;
  error.value = null;
  try {
    mergeScanResult(await api.collectInputs(paths, outputDirectory.value, null));
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    scanning.value = false;
  }
}

async function chooseFiles() {
  try {
    await addPaths(await api.pickFiles());
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function chooseFolder() {
  try {
    const [path] = await api.pickFolder();
    if (!path || scanning.value || activeBatch.value !== null) return;
    scanning.value = true;
    error.value = null;
    pendingFolderScan.value = null;
    const result = await api.collectInputs([path], outputDirectory.value, "both");
    const hasEncode = result.items.some((item) => accepts("font_to_woff2", item.conversion));
    const hasDecode = result.items.some((item) => accepts("woff2_to_font", item.conversion));
    if (hasEncode && hasDecode) {
      folderConversionMode.value = "both";
      pendingFolderScan.value = result;
      pendingFolder.value = path;
    } else {
      mergeScanResult(result);
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    scanning.value = false;
  }
}

function cancelFolderSelection() {
  pendingFolder.value = null;
  pendingFolderScan.value = null;
}

async function confirmFolderSelection() {
  const path = pendingFolder.value;
  if (!path || scanning.value || activeBatch.value !== null) return;
  pendingFolder.value = null;
  scanning.value = true;
  error.value = null;
  try {
    const cached = pendingFolderScan.value;
    pendingFolderScan.value = null;
    if (cached) {
      mergeScanResult({
        items: cached.items.filter((item) => accepts(folderConversionMode.value, item.conversion)),
        warnings: cached.warnings,
      });
    } else {
      mergeScanResult(await api.collectInputs([path], outputDirectory.value, folderConversionMode.value));
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    scanning.value = false;
  }
}

async function retargetOutputs(nextDirectory: string | null) {
  if (scanning.value || activeBatch.value !== null) return;
  outputDirectory.value = nextDirectory;
  const paths = items.value.map((item) => item.inputPath);
  if (paths.length === 0) return;
  scanning.value = true;
  error.value = null;
  try {
    const result = await api.collectInputs(paths, nextDirectory, null);
    items.value = result.items;
    warnings.value = result.warnings;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    scanning.value = false;
  }
}

async function chooseOutputFolder() {
  try {
    const [path] = await api.pickFolder();
    if (path) await retargetOutputs(path);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function start() {
  const pending = items.value.filter((item) => item.status === "queued");
  if (pending.length === 0) return;
  error.value = null;
  activeBatch.value = "";
  try {
    const batchId = await api.startConversion(pending, parallelism.value);
    if (activeBatch.value !== null) activeBatch.value = batchId;
  } catch (cause) {
    activeBatch.value = null;
    error.value = errorMessage(cause);
  }
}

async function cancel() {
  if (activeBatch.value === null) return;
  try {
    await api.cancelConversion(activeBatch.value);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function openOutput(path: string) {
  error.value = null;
  try {
    await api.openOutputFolder(path);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

function mergeScanResult(result: ScanResult) {
  const knownInputs = new Set(items.value.map((item) => item.inputPath));
  const knownOutputs = new Set(items.value.map((item) => item.outputPath.toLowerCase()));
  for (const item of result.items) {
    if (knownInputs.has(item.inputPath)) continue;
    knownInputs.add(item.inputPath);
    if (knownOutputs.has(item.outputPath.toLowerCase())) {
      item.status = "skipped";
      item.message = "Output path conflicts with another queued font";
    }
    knownOutputs.add(item.outputPath.toLowerCase());
    items.value.push(item);
  }
  const knownWarnings = new Set(warnings.value.map((warning) => `${warning.path}\0${warning.message}`));
  for (const warning of result.warnings) {
    const key = `${warning.path}\0${warning.message}`;
    if (!knownWarnings.has(key)) warnings.value.push(warning);
    knownWarnings.add(key);
  }
}

function startResize(index: number, event: PointerEvent) {
  event.preventDefault();
  resizing.value = { index, startX: event.clientX, startWidth: columnWidths.value[index] };
}

function resizeColumn(event: PointerEvent) {
  const current = resizing.value;
  if (!current) return;
  columnWidths.value[current.index] = Math.max(
    MIN_COLUMN_WIDTHS[current.index],
    current.startWidth + event.clientX - current.startX,
  );
}

function stopResize() {
  resizing.value = null;
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
</script>

<template>
  <main class="app-shell">
    <header class="topbar">
      <div class="brand">
        <div class="brand-mark">FC</div>
        <div><h1>Font Converter</h1><p>{{ t("tagline") }}</p></div>
      </div>
      <div class="topbar-actions">
        <div class="theme-switcher" role="group" :aria-label="t('theme')">
          <button type="button" :class="{ active: theme === 'system' }" @click="theme = 'system'">{{ t("themeSystem") }}</button>
          <button type="button" :class="{ active: theme === 'light' }" @click="theme = 'light'">{{ t("themeLight") }}</button>
          <button type="button" :class="{ active: theme === 'dark' }" @click="theme = 'dark'">{{ t("themeDark") }}</button>
        </div>
        <div class="language" :aria-label="t('language')">
          <button type="button" :class="{ active: locale === 'zh-CN' }" @click="locale = 'zh-CN'">中文</button>
          <button type="button" :class="{ active: locale === 'en' }" @click="locale = 'en'">EN</button>
        </div>
      </div>
    </header>

    <div class="workspace">
      <section class="drop-zone" :class="{ busy: scanning, dragging }">
        <div class="drop-content">
          <div class="drop-icon" aria-hidden="true">Aa</div>
          <h2>{{ t("dropTitle") }}</h2>
          <p class="drop-hint">{{ t("dropHint") }}</p>
          <div class="format-pills" :aria-label="t('supportedFormats')">
            <span class="format-pill">TTF / OTF → WOFF2</span>
            <span class="reverse-format">
              <span class="format-pill">WOFF2 → TTF / OTF</span>
              <span class="help-tooltip" tabindex="0" :aria-label="t('autoDetectHint')">?
                <span role="tooltip">{{ t("autoDetectHint") }}</span>
              </span>
            </span>
          </div>
          <div class="picker-actions">
            <button class="button secondary" type="button" :disabled="activeBatch !== null || scanning" @click="chooseFiles">{{ t("selectFiles") }}</button>
            <button class="button secondary" type="button" :disabled="activeBatch !== null || scanning" @click="chooseFolder">{{ t("selectFolder") }}</button>
          </div>
          <div class="output-destination">
            <button class="output-destination-copy" type="button" :title="t('chooseOutputFolder')" :disabled="activeBatch !== null || scanning" @click="chooseOutputFolder">
              <span>{{ t("outputFolder") }}</span>
              <strong :title="outputDirectory ?? t('sourceFolder')">{{ outputDirectory ? fileName(outputDirectory) : t("sourceFolder") }}</strong>
            </button>
            <div class="output-destination-actions">
              <button class="destination-button" type="button" :title="t('chooseOutputFolder')" :aria-label="t('chooseOutputFolder')" :disabled="activeBatch !== null || scanning" @click="chooseOutputFolder">
                <svg class="folder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.75 6.25h5.1l1.65 2h9.75c.97 0 1.75.78 1.75 1.75v7.75c0 .97-.78 1.75-1.75 1.75H3.75A1.75 1.75 0 0 1 2 17.75V8c0-.97.78-1.75 1.75-1.75Z"/><path d="M2.5 10h19"/></svg>
              </button>
              <button v-if="outputDirectory" class="destination-button reset" type="button" :title="t('resetOutputFolder')" :aria-label="t('resetOutputFolder')" :disabled="activeBatch !== null || scanning" @click="retargetOutputs(null)"><span aria-hidden="true">↺</span></button>
            </div>
          </div>
          <div class="scan-state" role="status" aria-live="polite">
            <span v-if="scanning" class="scanning-note"><span class="spinner" aria-hidden="true"></span>{{ t("scanning") }}</span>
          </div>
        </div>
      </section>

      <section class="queue-card" :aria-label="t('queue')">
        <div class="queue-toolbar">
          <div class="queue-overview">
            <div class="queue-title-line"><h2>{{ t("queue") }}</h2></div>
            <template v-if="summary.total > 0">
              <p class="completion" role="status" aria-live="polite">{{ t("completed") }} <strong>{{ finishedCount }}</strong> / <strong>{{ summary.total }}</strong></p>
              <div class="progress-track" aria-hidden="true"><span :style="{ width: `${progressPercent}%` }"></span></div>
              <div class="summary" aria-live="polite">
                <span v-if="summary.queued" class="summary-item queued">{{ t("queued") }} {{ summary.queued }}</span>
                <span v-if="summary.running" class="summary-item running">{{ t("running") }} {{ summary.running }}</span>
                <span v-if="summary.succeeded" class="summary-item succeeded">{{ t("succeeded") }} {{ summary.succeeded }}</span>
                <span v-if="summary.skipped" class="summary-item skipped">{{ t("skipped") }} {{ summary.skipped }}</span>
                <span v-if="summary.failed" class="summary-item failed">{{ t("failed") }} {{ summary.failed }}</span>
                <span v-if="summary.cancelled" class="summary-item cancelled">{{ t("cancelled") }} {{ summary.cancelled }}</span>
              </div>
            </template>
          </div>
          <div class="queue-controls">
            <label class="parallelism-control" :title="t('parallelismHint')">
              <span>{{ t("parallelism") }}</span>
              <input type="number" min="1" :max="MAX_PARALLELISM" step="1" :value="parallelism" :disabled="activeBatch !== null" :aria-label="t('parallelismHint')" @input="updateParallelism" />
            </label>
            <div class="queue-actions">
              <button v-if="activeBatch === null && finishedCount > 0" class="button ghost" type="button" @click="items = items.filter((item) => !isFinished(item.status))">{{ t("clearCompleted") }}</button>
              <button v-if="activeBatch === null && items.length > 0" class="button ghost" type="button" @click="items = []; warnings = []; error = null">{{ t("clearAll") }}</button>
              <button v-if="activeBatch === null" class="button primary" type="button" :disabled="pendingCount === 0 || scanning" @click="start">{{ t("start") }}<template v-if="pendingCount > 0"> ({{ pendingCount }})</template></button>
              <button v-else class="button danger" type="button" @click="cancel">{{ t("cancel") }}</button>
            </div>
          </div>
        </div>

        <div class="queue-notices">
          <div v-if="error" class="alert error" role="alert" aria-live="assertive"><strong>{{ t("commandFailed") }}</strong><span>{{ error }}</span></div>
          <details v-if="warnings.length" class="alert warnings">
            <summary>{{ t("warnings") }} ({{ warnings.length }})</summary>
            <ul><li v-for="warning in warnings" :key="`${warning.path}:${warning.message}`"><code>{{ warning.path }}</code> — {{ warning.message }}</li></ul>
          </details>
        </div>

        <div v-if="items.length === 0" class="empty-state"><h3>{{ t("emptyTitle") }}</h3><p>{{ t("emptyHint") }}</p></div>
        <div v-else class="queue-list" :style="queueColumnsStyle">
          <div class="queue-head">
            <span v-for="(heading, index) in [t('conversionDirection'), t('file'), t('path'), t('sizeChange'), t('status')]" :key="heading" class="column-heading" :class="{ 'size-change': index === 3 }">{{ heading }}<button class="column-resizer" type="button" :title="t('resizeColumn')" :aria-label="t('resizeColumn')" @pointerdown="startResize(index, $event)"></button></span>
            <span class="column-heading actions-heading">{{ t("actions") }}</span>
          </div>
          <article v-for="item in items" :key="item.id" class="queue-row">
            <div class="direction-cell"><div class="file-badge">{{ conversionBadge(item.conversion) }}</div></div>
            <div class="font-name-cell"><strong :title="displayPath(item.inputPath)">{{ fileName(item.inputPath) }}</strong></div>
            <div class="path-cell">
              <span :title="displayPath(item.inputPath)">{{ displayPath(item.inputPath) }}</span>
              <small :title="displayPath(item.outputPath)"><span aria-hidden="true">→ </span>{{ displayPath(item.outputPath) }}</small>
              <em v-if="item.message">{{ localizedMessage(locale, item.message) }}</em>
            </div>
            <div class="size-change"><span>{{ formatBytes(item.inputBytes) }} → {{ formatBytes(item.outputBytes) }}</span><strong>{{ formatSizeChange(item.inputBytes, item.outputBytes) }}</strong></div>
            <span class="status" :class="item.status" aria-live="polite">{{ t(statusKey(item.status)) }}</span>
            <div class="row-actions">
              <button class="open-output" type="button" :disabled="item.status !== 'succeeded'" :title="t('openOutputFolder')" :aria-label="t('openOutputFolder')" @click="openOutput(item.outputPath)">
                <svg class="folder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.75 6.25h5.1l1.65 2h9.75c.97 0 1.75.78 1.75 1.75v7.75c0 .97-.78 1.75-1.75 1.75H3.75A1.75 1.75 0 0 1 2 17.75V8c0-.97.78-1.75 1.75-1.75Z"/><path d="M2.5 10h19"/></svg>
              </button>
              <button class="remove-item" type="button" :disabled="activeBatch !== null" :title="t('remove')" :aria-label="t('remove')" @click="items = items.filter((candidate) => candidate.id !== item.id)"><span aria-hidden="true">×</span></button>
            </div>
          </article>
        </div>
      </section>
    </div>

    <div v-if="pendingFolder" class="folder-dialog-backdrop">
      <section class="folder-dialog" role="dialog" aria-modal="true" aria-labelledby="folder-dialog-title">
        <h2 id="folder-dialog-title">{{ t("folderDirectionTitle") }}</h2>
        <p class="folder-dialog-path" :title="pendingFolder">{{ fileName(pendingFolder) }}</p>
        <div class="folder-direction-options" role="radiogroup" :aria-label="t('conversionDirection')">
          <button v-for="option in [
            { value: 'font_to_woff2' as const, label: t('folderFontToWoff2') },
            { value: 'woff2_to_font' as const, label: t('folderWoff2ToFont') },
            { value: 'both' as const, label: t('folderBoth') },
          ]" :key="option.value" type="button" role="radio" class="folder-direction-option" :class="{ active: folderConversionMode === option.value }" :aria-checked="folderConversionMode === option.value" @click="folderConversionMode = option.value">{{ option.label }}</button>
        </div>
        <div class="folder-dialog-actions">
          <button class="button ghost" type="button" @click="cancelFolderSelection">{{ t("cancel") }}</button>
          <button class="button primary" type="button" @click="confirmFolderSelection">{{ t("scanFolder") }}</button>
        </div>
      </section>
    </div>
  </main>
</template>
