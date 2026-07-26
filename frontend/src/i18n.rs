use web_sys::window;

const STORAGE_KEY: &str = "ttf2woff2-gui.locale";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Locale {
    ZhCn,
    En,
}

#[derive(Clone, Copy)]
pub enum Message {
    Tagline,
    DropTitle,
    DropHint,
    SelectFiles,
    SelectFolder,
    Start,
    Cancel,
    ClearCompleted,
    EmptyTitle,
    EmptyHint,
    File,
    Output,
    Size,
    Status,
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
    SafeOutput,
    Language,
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
            (Self::ZhCn, Message::Tagline) => "快速、安全地将 TrueType 字体转换为 WOFF2",
            (Self::ZhCn, Message::DropTitle) => "拖放字体或文件夹到这里",
            (Self::ZhCn, Message::DropHint) => "递归扫描 .ttf；输出保存在源文件旁",
            (Self::ZhCn, Message::SelectFiles) => "选择文件",
            (Self::ZhCn, Message::SelectFolder) => "选择文件夹",
            (Self::ZhCn, Message::Start) => "开始转换",
            (Self::ZhCn, Message::Cancel) => "取消",
            (Self::ZhCn, Message::ClearCompleted) => "清除已完成",
            (Self::ZhCn, Message::EmptyTitle) => "转换队列为空",
            (Self::ZhCn, Message::EmptyHint) => "添加一个或多个 TTF 文件开始使用",
            (Self::ZhCn, Message::File) => "源文件",
            (Self::ZhCn, Message::Output) => "输出",
            (Self::ZhCn, Message::Size) => "大小",
            (Self::ZhCn, Message::Status) => "状态",
            (Self::ZhCn, Message::Queued) => "等待",
            (Self::ZhCn, Message::Running) => "转换中",
            (Self::ZhCn, Message::Succeeded) => "成功",
            (Self::ZhCn, Message::Skipped) => "已跳过",
            (Self::ZhCn, Message::Failed) => "失败",
            (Self::ZhCn, Message::Cancelled) => "已取消",
            (Self::ZhCn, Message::Total) => "总计",
            (Self::ZhCn, Message::Warnings) => "扫描提示",
            (Self::ZhCn, Message::NoFonts) => "没有发现可转换的 TTF 文件",
            (Self::ZhCn, Message::CommandFailed) => "操作失败",
            (Self::ZhCn, Message::SafeOutput) => "不会覆盖已有 WOFF2 文件",
            (Self::ZhCn, Message::Language) => "语言",

            (Self::En, Message::Tagline) => "Fast, safe TrueType to WOFF2 conversion",
            (Self::En, Message::DropTitle) => "Drop fonts or folders here",
            (Self::En, Message::DropHint) => "Scans .ttf recursively; saves beside each source",
            (Self::En, Message::SelectFiles) => "Select files",
            (Self::En, Message::SelectFolder) => "Select folder",
            (Self::En, Message::Start) => "Start conversion",
            (Self::En, Message::Cancel) => "Cancel",
            (Self::En, Message::ClearCompleted) => "Clear completed",
            (Self::En, Message::EmptyTitle) => "The conversion queue is empty",
            (Self::En, Message::EmptyHint) => "Add one or more TTF files to get started",
            (Self::En, Message::File) => "Source",
            (Self::En, Message::Output) => "Output",
            (Self::En, Message::Size) => "Size",
            (Self::En, Message::Status) => "Status",
            (Self::En, Message::Queued) => "Queued",
            (Self::En, Message::Running) => "Converting",
            (Self::En, Message::Succeeded) => "Succeeded",
            (Self::En, Message::Skipped) => "Skipped",
            (Self::En, Message::Failed) => "Failed",
            (Self::En, Message::Cancelled) => "Cancelled",
            (Self::En, Message::Total) => "Total",
            (Self::En, Message::Warnings) => "Scan notices",
            (Self::En, Message::NoFonts) => "No convertible TTF files were found",
            (Self::En, Message::CommandFailed) => "Operation failed",
            (Self::En, Message::SafeOutput) => "Existing WOFF2 files are never overwritten",
            (Self::En, Message::Language) => "Language",
        }
    }
}
