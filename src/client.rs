use std::time::Duration;

use crate::error::ReportError;
use crate::model::HardwareReport;

const ATTEMPTS: u32 = 4;
const BASE_DELAY: Duration = Duration::from_secs(2);

/// Posts a [`HardwareReport`] as JSON to the controller.
pub struct ControllerClient {
    http: reqwest::Client,
    url: String,
    token: Option<String>,
}

impl ControllerClient {
    pub fn new(url: String, token: Option<String>, timeout: Duration) -> Result<Self, ReportError> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("failed to install rustls crypto provider");
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(concat!("hwscan/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http, url, token })
    }

    /// POST with exponential backoff on transport errors and 5xx.
    pub async fn submit(&self, report: &HardwareReport) -> Result<(), ReportError> {
        let mut delay = BASE_DELAY;
        for attempt in 1..=ATTEMPTS {
            match self.send_once(report).await {
                Ok(()) => return Ok(()),
                Err(err) if err.is_retryable() && attempt < ATTEMPTS => {
                    tracing::warn!(attempt, %err, retry_in = ?delay, "report failed, retrying");
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
                Err(err) => return Err(err),
            }
        }
        unreachable!("loop returns on the last attempt")
    }

    async fn send_once(&self, report: &HardwareReport) -> Result<(), ReportError> {
        let mut req = self.http.post(&self.url).json(report);
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;
        let status = resp.status();
        if status.is_success() {
            tracing::info!(status = status.as_u16(), "controller accepted report");
            return Ok(());
        }

        let body = resp.text().await.unwrap_or_default();
        Err(ReportError::Rejected {
            status: status.as_u16(),
            body: body.chars().take(500).collect(),
        })
    }
}
