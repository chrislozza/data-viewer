variable "aws_region" {
  description = "AWS region to deploy resources"
  type        = string
  default     = "us-east-1"
}

variable "lambda_function_name" {
  description = "Name of the Lambda function"
  type        = string
  default     = "oauth-proxy"
}

variable "lambda_handler" {
  description = "Handler for the Lambda function"
  type        = string
  default     = "oauth"
}

variable "lambda_runtime" {
  description = "Runtime for the Lambda function"
  type        = string
  default     = "provided.al2"
}

variable "lambda_package_path" {
  description = "Path to the Lambda deployment package"
  type        = string
}

variable "lambda_environment_variables" {
  description = "Environment variables for the Lambda function"
  type        = map(string)
  default     = {}
}

variable "api_name" {
  description = "Name of the API Gateway"
  type        = string
  default     = "oauth-proxy-api"
}

variable "api_route_key" {
  description = "Route key for the API Gateway"
  type        = string
  default     = "POST /oauth/token"
}
