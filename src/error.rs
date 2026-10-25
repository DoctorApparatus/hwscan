use std::path::PathBuf;

/// Errors raised while probing the local machine.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse {what}: {reason}")]
    Parse { what: &'static str, reason: String },

    #[error("probe task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

/// Errors raised while talking to the controller.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("controller request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("controller rejected report: HTTP {status}: {body}")]
    Rejected { status: u16, body: String },
}

impl ReportError {
    /// Worth retrying: transport failures and 5xx responses.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(_) => true,
            Self::Rejected { status, .. } => *status >= 500,
        }
    }
}
