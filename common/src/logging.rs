use anyhow::Result;
use std::str::FromStr;
use tracing::Level;

#[derive(Debug, Clone)]
pub struct Logging {}

impl Logging {
    pub async fn new(log_level: &str) -> Result<Self> {
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
