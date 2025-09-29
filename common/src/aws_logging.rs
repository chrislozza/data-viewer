use aws_sdk_cloudwatchlogs::{Client as CloudWatchClient, types::InputLogEvent};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;


#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Message {
    Text(String),
    Json(serde_json::Value),
}

#[derive(Debug, Serialize)]
struct LogEvent {
    level: String,
    message: Message,
}

// Simple struct to hold log data
#[derive(Debug)]
struct LogEntry {
    timestamp: i64,
    message: String,
    level: String,
}

// Our CloudWatch Layer
struct CloudWatchLayer {
    sender: mpsc::Sender<LogEntry>,
}

impl CloudWatchLayer {
    pub fn new(log_group: String, log_stream: String, batch_size: usize) -> Self {
        let (sender, mut receiver) = mpsc::channel::<LogEntry>(100);

        // Spawn background task to send logs to CloudWatch
        tokio::spawn(async move {
            // Initialize AWS client
            let config = aws_config::from_env().load().await;

            let client = CloudWatchClient::new(&config);

            // Create log group and stream if they don't exist
            // Error handling omitted for brevity
            let _ = client.create_log_group().log_group_name(&log_group).send().await;

            let _ = client
                .create_log_stream()
                .log_group_name(&log_group)
                .log_stream_name(&log_stream)
                .send()
                .await;

            let mut log_batch = Vec::new();
            let mut sequence_token = None;

            // Main processing loop
            loop {
                tokio::select! {
                    Some(entry) = receiver.recv() => {
                        // Format the log message with level
                        let message = serde_json::from_str(&entry.message)
                            .map(Message::Json)
                            .unwrap_or_else(|_| Message::Text(entry.message.clone()));

                        let log_event = LogEvent {
                            level: entry.level.clone(),
                            message,
                        };

                        let formatted_message = to_string(&log_event).unwrap();

                        log_batch.push(
                            InputLogEvent::builder()
                                .timestamp(entry.timestamp)
                                .message(formatted_message)
                                .build().unwrap(),
                        );

                        // Send batch if it reaches the specified size
                        if log_batch.len() >= batch_size {
                            send_logs(&client, &log_group, &log_stream, &mut log_batch, &mut sequence_token).await;
                        }
                    }
                    _ = sleep(Duration::from_secs(5)) => {
                        // Flush any pending logs after timeout
                        if !log_batch.is_empty() {
                            send_logs(&client, &log_group, &log_stream, &mut log_batch, &mut sequence_token).await;
                        }
                    }
                }
            }
        });

        CloudWatchLayer { sender }
    }
}

// Helper function to send logs to CloudWatch
async fn send_logs(
    client: &CloudWatchClient,
    log_group: &str,
    log_stream: &str,
    log_batch: &mut Vec<InputLogEvent>,
    sequence_token: &mut Option<String>,
) {
    if log_batch.is_empty() {
        return;
    }

    // Build request with or without sequence token
    let mut request = client
        .put_log_events()
        .log_group_name(log_group)
        .log_stream_name(log_stream)
        .set_log_events(Some(log_batch.clone()));

    if let Some(token) = sequence_token {
        request = request.sequence_token(token.clone());
    }

    match request.send().await {
        Ok(response) => {
            // Save the sequence token for next batch
            *sequence_token = response.next_sequence_token;
            log_batch.clear();
        }
        Err(err) => {
            eprintln!("Error sending logs to CloudWatch: {:?}", err);
            // You could add retry logic here, or handle specific AWS errors
            log_batch.clear();
        }
    }
}

// Visitor to extract log fields
struct LogVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }
}

// Implement Layer trait for CloudWatchLayer
impl<S> Layer<S> for CloudWatchLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Extract log level
        let level = event.metadata().level().as_str();

        // Extract message
        let mut visitor = LogVisitor { message: None };
        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            // Get current timestamp in milliseconds
            let timestamp = Utc::now().timestamp_millis();

            // Send log entry to processing task
            let _ = self.sender.try_send(LogEntry {
                timestamp,
                message,
                level: level.to_string(),
            });
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub log_group: String,
    pub log_stream: String,
    pub level: String,
}

// Function to set up the tracing subscriber with CloudWatch
pub fn init_cloudwatch_logger(settings: &LoggingConfig) -> anyhow::Result<()> {
    let cloudwatch_layer = CloudWatchLayer::new(
        settings.log_group.to_string(),
        settings.log_stream.to_string(),
        10,
    );

    // Create console output layer
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);

    let level = tracing::Level::from_str(&settings.level).unwrap_or(tracing::Level::INFO);
    // Register both layers with the subscriber
    let _ = tracing_subscriber::registry()
        .with(cloudwatch_layer.with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)))
        .with(fmt_layer.with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)))
        .try_init();

    Ok(())
}
