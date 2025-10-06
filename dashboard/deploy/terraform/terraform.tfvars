# Default environment configuration for the dashboard App Runner service
# Update values as needed for your deployment.

aws_region = "us-east-1"
app_name   = "data-viewer-dashboard"

# Compute and scaling
cpu             = "0.5 vCPU"
memory          = "1 GB"
min_size        = 1
max_size        = 5
max_concurrency = 150

# Application runtime
app_port           = "8080"
health_check_path  = "/health"
start_command      = "/bin/bash ./startup.sh"
auto_deployments_enabled = true

# Non-sensitive environment variables (sensitive values should come from SSM or Secrets Manager)
environment_variables = {
  RUST_LOG                      = "info"
  PORT                          = "8080"
  CLOUD_SQL_CONNECTION_STRING   = "savvy-nimbus-306111:europe-west2:vibgo-sql"
  CLOUD_SQL_PORT                = "5433"
}

tags = {
  Environment = "production"
  Project     = "data-viewer"
  Component   = "dashboard"
}
