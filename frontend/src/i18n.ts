export type Locale = "zh-CN" | "en";
export type Theme = "system" | "light" | "dark";

const messages = {
  "zh-CN": {
    tagline: "轻松搞定TTF/OTF和WOFF2字体格式互转",
    dropTitle: "拖放字体或文件夹到这里",
    dropHint: "支持 .ttf、.otf 和 .woff2",
    selectFiles: "选择文件",
    selectFolder: "选择文件夹",
    scanning: "正在扫描字体…",
    outputFolder: "输出文件夹",
    sourceFolder: "源文件所在文件夹（默认）",
    chooseOutputFolder: "选择输出文件夹",
    resetOutputFolder: "恢复到源文件夹",
    start: "开始转换",
    cancel: "取消",
    clearCompleted: "清除已完成",
    clearAll: "全部清除",
    queue: "转换队列",
    completed: "已完成",
    emptyTitle: "尚未添加字体",
    emptyHint: "添加后将在这里显示转换方向、文件大小、输出路径和处理状态",
    file: "字体名称",
    path: "路径",
    sizeChange: "体积变化",
    status: "状态",
    actions: "操作",
    remove: "移除",
    openOutputFolder: "打开输出文件夹",
    queued: "等待",
    running: "转换中",
    succeeded: "成功",
    skipped: "已跳过",
    failed: "失败",
    cancelled: "已取消",
    warnings: "扫描提示",
    commandFailed: "操作失败",
    supportedFormats: "支持的转换格式",
    autoDetectHint: "WOFF2→TTF/OTF时，FontConverter 会根据WOFF2中字体轮廓类型信息将其自动转换为TTF或 OTF。\nTTF/OTF→WOFF2通常需要十几秒。",
    folderDirectionTitle: "选择文件中字体的转换方向",
    folderFontToWoff2: "TTF / OTF → WOFF2",
    folderWoff2ToFont: "WOFF2 → TTF / OTF",
    folderBoth: "两种方向都转换",
    scanFolder: "扫描文件夹",
    conversionDirection: "转换方向",
    resizeColumn: "拖动调整列宽",
    language: "语言",
    theme: "主题",
    themeSystem: "跟随系统",
    themeLight: "浅色",
    themeDark: "深色",
    parallelism: "并行数",
    parallelismHint: "同时转换的字体数量（1–32）",
  },
  en: {
    tagline: "Effortless TTF/OTF and WOFF2 font conversion",
    dropTitle: "Drop fonts or folders here",
    dropHint: "Supports .ttf, .otf, and .woff2",
    selectFiles: "Select files",
    selectFolder: "Select folder",
    scanning: "Scanning fonts…",
    outputFolder: "Output folder",
    sourceFolder: "Beside each source (default)",
    chooseOutputFolder: "Choose output folder",
    resetOutputFolder: "Use source folders",
    start: "Start conversion",
    cancel: "Cancel",
    clearCompleted: "Clear completed",
    clearAll: "Clear all",
    queue: "Conversion queue",
    completed: "Completed",
    emptyTitle: "No fonts added yet",
    emptyHint: "Added fonts will show their direction, sizes, output paths, and status here",
    file: "Font name",
    path: "Path",
    sizeChange: "Size change",
    status: "Status",
    actions: "Actions",
    remove: "Remove",
    openOutputFolder: "Open output folder",
    queued: "Queued",
    running: "Converting",
    succeeded: "Succeeded",
    skipped: "Skipped",
    failed: "Failed",
    cancelled: "Cancelled",
    warnings: "Scan notices",
    commandFailed: "Operation failed",
    supportedFormats: "Supported conversion formats",
    autoDetectHint: "WOFF2→TTF/OTF uses the outline type stored in WOFF2 to restore TTF or OTF.\nTTF/OTF→WOFF2 usually takes over ten seconds.",
    folderDirectionTitle: "Choose a conversion direction for the fonts in the folder",
    folderFontToWoff2: "TTF / OTF → WOFF2",
    folderWoff2ToFont: "WOFF2 → TTF / OTF",
    folderBoth: "Convert both directions",
    scanFolder: "Scan folder",
    conversionDirection: "Direction",
    resizeColumn: "Drag to resize column",
    language: "Language",
    theme: "Theme",
    themeSystem: "System",
    themeLight: "Light",
    themeDark: "Dark",
    parallelism: "Parallelism",
    parallelismHint: "Fonts converted at the same time (1–32)",
  },
} as const;

export type MessageKey = keyof (typeof messages)["en"];

export function translate(locale: Locale, key: MessageKey): string {
  return messages[locale][key];
}

export function loadLocale(): Locale {
  const saved = localStorage.getItem("font-converter.locale");
  if (saved === "zh-CN" || saved === "en") return saved;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function loadTheme(): Theme {
  const saved = localStorage.getItem("font-converter.theme");
  return saved === "light" || saved === "dark" ? saved : "system";
}
