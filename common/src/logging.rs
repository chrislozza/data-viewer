use anyhow::Result;
use std::str::FromStr;
use tracing::Level;

#[derive(Debug, Clone)]
pub struct Logging {}

impl Logging {
    pub fn new(log_level: &str) -> Result<Self> {
        let level = Level::from_str(log_level).unwrap();

        let subscriber = tracing_subscriber::fmt()
            // Display source code file paths
            .with_file(true)
            // Display source code line numbers
            .with_line_number(true)
            // Display the thread ID an event was recorded on
            .with_thread_ids(true)
            // Don't display the event's target (module path)
            .with_target(false)
            // Assign a log-level
            .with_max_level(level)
            // Use a more compact, abbreviated log format
            .compact()
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;
        Ok(Logging {})
    }
}

pub struct StructuredLogging {}

impl StructuredLogging {
    pub fn new() -> Result<Self> {
        tracing_subscriber::fmt()
            .json()
            .with_max_level(tracing::Level::INFO)
            // this needs to be set to remove duplicated information in the log.
            .with_current_span(false)
            // this needs to be set to false, otherwise ANSI color codes will
            // show up in a confusing manner in CloudWatch logs.
            .with_ansi(false)
            // disabling time is handy because CloudWatch will add the ingestion time.
            .without_time()
            // remove the name of the function from every log entry
            .with_target(false)
            .init();
        Ok(StructuredLogging {})
    }
}
