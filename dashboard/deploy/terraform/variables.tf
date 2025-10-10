variable "aws_region" {
  description = "AWS region to deploy resources"
  type        = string
  default     = "us-east-1"
}

variable "app_name" {
  description = "Name of the application"
  type        = string
  default     = "data-viewer-dashboard"
}

variable "app_port" {
  description = "Port the application listens on"
  type        = string
  default     = "8080"
}

variable "cpu" {
  description = "CPU units for the App Runner service (0.25 vCPU, 0.5 vCPU, 1 vCPU, 2 vCPU, 4 vCPU)"
  type        = string
  default     = "0.25 vCPU"
}

variable "memory" {
  description = "Memory for the App Runner service (0.5 GB, 1 GB, 2 GB, 3 GB, 4 GB, 6 GB, 8 GB, 10 GB, 12 GB)"
  type        = string
  default     = "0.5 GB"
}

variable "min_size" {
  description = "Minimum number of instances"
  type        = number
  default     = 1
}

variable "max_size" {
  description = "Maximum number of instances"
  type        = number
  default     = 10
}

variable "max_concurrency" {
  description = "Maximum number of concurrent requests per instance"
  type        = number
  default     = 100
}

variable "auto_deployments_enabled" {
  description = "Enable automatic deployments when a new image is pushed"
  type        = bool
  default     = true
}

variable "health_check_path" {
  description = "Path for health check"
  type        = string
  default     = "/"
}

variable "start_command" {
  description = "Command to start the application"
  type        = string
  default     = "/bin/bash ./startup.sh"
}

variable "db_password" {
  description = "Database password (should be passed via environment variable or tfvars)"
  type        = string
  sensitive   = true
  default     = ""
}

variable "environment_variables" {
  description = "Environment variables for the application"
  type        = map(string)
  default = {
    RUST_LOG                      = "info"
    PORT                          = "8080"
    CLOUD_SQL_CONNECTION_STRING   = "savvy-nimbus-306111:europe-west2:vibgo-sql"
    CLOUD_SQL_PORT                = "5433"
  }
}

variable "config_bucket_name" {
  description = "S3 bucket name for configuration files"
  type        = string
  default     = "data-viewer-config"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default = {
    Environment = "production"
    Project     = "data-viewer"
    Component   = "dashboard"
  }
}
