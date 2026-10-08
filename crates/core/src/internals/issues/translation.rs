use serde::Serialize;

use super::{IssueEvent, IssueKind};

/// Translation problems that can be tracked as issues.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranslationIssue {
    MissingTranslation { locale: String, key: String },
    MissingLocale { locale: String },
}

impl IssueEvent for TranslationIssue {
    fn kind(&self) -> IssueKind {
        match self {
            TranslationIssue::MissingTranslation { .. } => IssueKind::new("missing_translation"),
            TranslationIssue::MissingLocale { .. } => IssueKind::new("missing_locale"),
        }
    }

    fn payload(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("a TranslationIssue is always serializable")
    }

    fn summary(&self) -> String {
        match self {
            TranslationIssue::MissingTranslation { locale, key } => {
                format!("Missing translation for locale '{locale}' and key '{key}'")
            }
            TranslationIssue::MissingLocale { locale } => {
                format!("Missing locale '{locale}'")
            }
        }
    }
}
