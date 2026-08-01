use web_sys::window;

const STORAGE_KEY: &str = "font-converter.locale";
const THEME_STORAGE_KEY: &str = "font-converter.theme";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Locale {
    ZhCn,
    En,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn load() -> Self {
        let Some(value) = window()
            .and_then(|window| window.local_storage().ok().flatten())
            .and_then(|storage| storage.get_item(THEME_STORAGE_KEY).ok().flatten())
        else {
            return Self::System;
        };

        match value.as_str() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn save(self) {
        if let Some(storage) = window().and_then(|window| window.local_storage().ok().flatten()) {
            let _ = storage.set_item(THEME_STORAGE_KEY, self.as_str());
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn apply(self) {
        if let Some(document) = window().and_then(|window| window.document())
            && let Some(root) = document.document_element()
        {
            let _ = root.set_attribute("data-theme", self.as_str());
        }
    }
}

#[derive(Clone, Copy)]
pub enum Message {
    Tagline,
    DropTitle,
    DropHint,
    AddFonts,
    SelectFiles,
    SelectFilesHint,
    SelectFolder,
    SelectFolderHint,
    Scanning,
    OutputFolder,
    SourceFolder,
    ChooseOutputFolder,
    ResetOutputFolder,
    Start,
    Cancel,
    ClearCompleted,
    ClearAll,
    Queue,
    ConversionTimeHint,
    Completed,
    EmptyTitle,
    EmptyHint,
    File,
    Path,
    Output,
    InputSize,
    OutputSize,
    SizeChange,
    Status,
    Actions,
    Remove,
    OpenOutputFolder,
    Queued,
    Running,
    Succeeded,
    Skipped,
    Failed,
    Cancelled,
    Total,
    Warnings,
    NoFonts,
    CommandFailed,
    SupportedFormats,
    AutoDetectHint,
    ConversionDirection,
    ResizeColumn,
    Language,
    Theme,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
}

impl Locale {
    pub fn load() -> Self {
        if let Some(storage) = window().and_then(|window| window.local_storage().ok().flatten())
            && let Ok(Some(value)) = storage.get_item(STORAGE_KEY)
        {
            return if value == "zh-CN" {
                Self::ZhCn
            } else {
                Self::En
            };
        }

        let language = window()
            .and_then(|window| window.navigator().language())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if language.starts_with("zh") {
            Self::ZhCn
        } else {
            Self::En
        }
    }

    pub fn save(self) {
        if let Some(storage) = window().and_then(|window| window.local_storage().ok().flatten()) {
            let _ = storage.set_item(STORAGE_KEY, if self == Self::ZhCn { "zh-CN" } else { "en" });
        }
    }

    pub fn t(self, message: Message) -> &'static str {
        match (self, message) {
            (Self::ZhCn, Message::Tagline) => "轻松搞定TTF/OTF和WOFF2字体格式互转",
            (Self::ZhCn, Message::DropTitle) => "拖放字体或文件夹到这里",
            (Self::ZhCn, Message::DropHint) => "递归扫描 .ttf、.otf 和 .woff2；自动识别转换方向",
            (Self::ZhCn, Message::AddFonts) => "选择文件/文件夹",
            (Self::ZhCn, Message::SelectFiles) => "选择文件",
            (Self::ZhCn, Message::SelectFilesHint) => "添加一个或多个字体文件",
            (Self::ZhCn, Message::SelectFolder) => "选择文件夹",
            (Self::ZhCn, Message::SelectFolderHint) => "递归扫描文件夹中的字体",
            (Self::ZhCn, Message::Scanning) => "正在扫描字体…",
            (Self::ZhCn, Message::OutputFolder) => "输出文件夹",
            (Self::ZhCn, Message::SourceFolder) => "源文件所在文件夹（默认）",
            (Self::ZhCn, Message::ChooseOutputFolder) => "选择输出文件夹",
            (Self::ZhCn, Message::ResetOutputFolder) => "恢复到源文件夹",
            (Self::ZhCn, Message::Start) => "开始转换",
            (Self::ZhCn, Message::Cancel) => "取消",
            (Self::ZhCn, Message::ClearCompleted) => "清除已完成",
            (Self::ZhCn, Message::ClearAll) => "全部清除",
            (Self::ZhCn, Message::Queue) => "转换队列",
            (Self::ZhCn, Message::ConversionTimeHint) => "TTF/OTF→WOFF2通常需要十余秒",
            (Self::ZhCn, Message::Completed) => "已完成",
            (Self::ZhCn, Message::EmptyTitle) => "尚未添加字体",
            (Self::ZhCn, Message::EmptyHint) => "从左侧添加或直接拖入字体",
            (Self::ZhCn, Message::File) => "字体名称",
            (Self::ZhCn, Message::Path) => "路径",
            (Self::ZhCn, Message::Output) => "输出",
            (Self::ZhCn, Message::InputSize) => "原始大小",
            (Self::ZhCn, Message::OutputSize) => "转换后大小",
            (Self::ZhCn, Message::SizeChange) => "体积变化",
            (Self::ZhCn, Message::Status) => "状态",
            (Self::ZhCn, Message::Actions) => "操作",
            (Self::ZhCn, Message::Remove) => "移除",
            (Self::ZhCn, Message::OpenOutputFolder) => "打开输出文件夹",
            (Self::ZhCn, Message::Queued) => "等待",
            (Self::ZhCn, Message::Running) => "转换中",
            (Self::ZhCn, Message::Succeeded) => "成功",
            (Self::ZhCn, Message::Skipped) => "已跳过",
            (Self::ZhCn, Message::Failed) => "失败",
            (Self::ZhCn, Message::Cancelled) => "已取消",
            (Self::ZhCn, Message::Total) => "总计",
            (Self::ZhCn, Message::Warnings) => "扫描提示",
            (Self::ZhCn, Message::NoFonts) => "没有发现可转换的字体文件",
            (Self::ZhCn, Message::CommandFailed) => "操作失败",
            (Self::ZhCn, Message::SupportedFormats) => "支持的转换格式",
            (Self::ZhCn, Message::AutoDetectHint) => {
                "WOFF2 文件包含字体轮廓类型信息，Font Converter 会据此自动转换为 TTF（TrueType）或 OTF（CFF/OpenType）。"
            }
            (Self::ZhCn, Message::ConversionDirection) => "转换方向",
            (Self::ZhCn, Message::ResizeColumn) => "拖动调整列宽",
            (Self::ZhCn, Message::Language) => "语言",
            (Self::ZhCn, Message::Theme) => "主题",
            (Self::ZhCn, Message::ThemeSystem) => "跟随系统",
            (Self::ZhCn, Message::ThemeLight) => "浅色",
            (Self::ZhCn, Message::ThemeDark) => "深色",

            (Self::En, Message::Tagline) => "Effortless TTF/OTF and WOFF2 font conversion",
            (Self::En, Message::DropTitle) => "Drop fonts or folders here",
            (Self::En, Message::DropHint) => {
                "Scans .ttf, .otf, and .woff2 recursively; detects direction automatically"
            }
            (Self::En, Message::AddFonts) => "Select files/folder",
            (Self::En, Message::SelectFiles) => "Select files",
            (Self::En, Message::SelectFilesHint) => "Add one or more font files",
            (Self::En, Message::SelectFolder) => "Select folder",
            (Self::En, Message::SelectFolderHint) => "Scan fonts in a folder recursively",
            (Self::En, Message::Scanning) => "Scanning fonts…",
            (Self::En, Message::OutputFolder) => "Output folder",
            (Self::En, Message::SourceFolder) => "Beside each source (default)",
            (Self::En, Message::ChooseOutputFolder) => "Choose output folder",
            (Self::En, Message::ResetOutputFolder) => "Use source folders",
            (Self::En, Message::Start) => "Start conversion",
            (Self::En, Message::Cancel) => "Cancel",
            (Self::En, Message::ClearCompleted) => "Clear completed",
            (Self::En, Message::ClearAll) => "Clear all",
            (Self::En, Message::Queue) => "Conversion queue",
            (Self::En, Message::ConversionTimeHint) => {
                "TTF/OTF → WOFF2 usually takes over ten seconds"
            }
            (Self::En, Message::Completed) => "Completed",
            (Self::En, Message::EmptyTitle) => "No fonts added yet",
            (Self::En, Message::EmptyHint) => "Add fonts on the left or drop them here",
            (Self::En, Message::File) => "Font name",
            (Self::En, Message::Path) => "Path",
            (Self::En, Message::Output) => "Output",
            (Self::En, Message::InputSize) => "Input size",
            (Self::En, Message::OutputSize) => "Output size",
            (Self::En, Message::SizeChange) => "Size change",
            (Self::En, Message::Status) => "Status",
            (Self::En, Message::Actions) => "Actions",
            (Self::En, Message::Remove) => "Remove",
            (Self::En, Message::OpenOutputFolder) => "Open output folder",
            (Self::En, Message::Queued) => "Queued",
            (Self::En, Message::Running) => "Converting",
            (Self::En, Message::Succeeded) => "Succeeded",
            (Self::En, Message::Skipped) => "Skipped",
            (Self::En, Message::Failed) => "Failed",
            (Self::En, Message::Cancelled) => "Cancelled",
            (Self::En, Message::Total) => "Total",
            (Self::En, Message::Warnings) => "Scan notices",
            (Self::En, Message::NoFonts) => "No convertible font files were found",
            (Self::En, Message::CommandFailed) => "Operation failed",
            (Self::En, Message::SupportedFormats) => "Supported conversion formats",
            (Self::En, Message::AutoDetectHint) => {
                "WOFF2 stores its font outline type. Font Converter uses it to restore TTF (TrueType) or OTF (CFF/OpenType) automatically."
            }
            (Self::En, Message::ConversionDirection) => "Direction",
            (Self::En, Message::ResizeColumn) => "Drag to resize column",
            (Self::En, Message::Language) => "Language",
            (Self::En, Message::Theme) => "Theme",
            (Self::En, Message::ThemeSystem) => "System",
            (Self::En, Message::ThemeLight) => "Light",
            (Self::En, Message::ThemeDark) => "Dark",
        }
    }
}
