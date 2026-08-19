use serde::{Deserialize, Serialize};

pub const PROGRESS_EVENT: &str = "conversion-progress";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionKind {
    TtfToWoff2,
    OtfToWoff2,
    Woff2ToTtf,
    Woff2ToOtf,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderConversionMode {
    FontToWoff2,
    Woff2ToFont,
    Both,
}

impl FolderConversionMode {
    pub fn accepts(self, conversion: ConversionKind) -> bool {
        match self {
            Self::FontToWoff2 => {
                matches!(
                    conversion,
                    ConversionKind::TtfToWoff2 | ConversionKind::OtfToWoff2
                )
            }
            Self::Woff2ToFont => {
                matches!(
                    conversion,
                    ConversionKind::Woff2ToTtf | ConversionKind::Woff2ToOtf
                )
            }
            Self::Both => true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Queued,
    Running,
    Succeeded,
    Skipped,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InputNotFound,
    InputUnreadable,
    InvalidFont,
    UnsupportedFormat,
    OutputExists,
    OutputConflict,
    OutputUnwritable,
    InputTooLarge,
    ConversionFailed,
    Cancelled,
}

impl ItemStatus {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Skipped | Self::Failed | Self::Cancelled
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub conversion: ConversionKind,
    pub input_path: String,
    pub output_path: String,
    pub input_bytes: Option<u64>,
    pub output_bytes: Option<u64>,
    pub status: ItemStatus,
    pub error_code: Option<ErrorCode>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanWarning {
    pub path: String,
    pub error_code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub items: Vec<QueueItem>,
    pub warnings: Vec<ScanWarning>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub total: usize,
    pub queued: usize,
    pub running: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: usize,
}

impl BatchSummary {
    pub fn from_items(items: &[QueueItem]) -> Self {
        let mut summary = Self {
            total: items.len(),
            ..Self::default()
        };
        for item in items {
            match item.status {
                ItemStatus::Queued => summary.queued += 1,
                ItemStatus::Running => summary.running += 1,
                ItemStatus::Succeeded => summary.succeeded += 1,
                ItemStatus::Skipped => summary.skipped += 1,
                ItemStatus::Failed => summary.failed += 1,
                ItemStatus::Cancelled => summary.cancelled += 1,
            }
        }
        summary
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub batch_id: String,
    pub item: Option<QueueItem>,
    pub summary: BatchSummary,
    pub finished: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_every_status() {
        let statuses = [
            ItemStatus::Queued,
            ItemStatus::Running,
            ItemStatus::Succeeded,
            ItemStatus::Skipped,
            ItemStatus::Failed,
            ItemStatus::Cancelled,
        ];
        let items = statuses
            .into_iter()
            .enumerate()
            .map(|(index, status)| QueueItem {
                id: index.to_string(),
                conversion: ConversionKind::TtfToWoff2,
                input_path: String::new(),
                output_path: String::new(),
                input_bytes: None,
                output_bytes: None,
                status,
                error_code: None,
                message: None,
            })
            .collect::<Vec<_>>();

        let summary = BatchSummary::from_items(&items);
        assert_eq!(summary.total, 6);
        assert_eq!(summary.queued, 1);
        assert_eq!(summary.running, 1);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.cancelled, 1);
    }
}
