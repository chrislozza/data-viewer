output "app_runner_service_url" {
  description = "The URL of the App Runner service"
  value       = "https://${aws_apprunner_service.dashboard_service.service_url}"
}

output "app_runner_service_id" {
  description = "The ID of the App Runner service"
  value       = aws_apprunner_service.dashboard_service.service_id
}

output "app_runner_service_arn" {
  description = "The ARN of the App Runner service"
  value       = aws_apprunner_service.dashboard_service.arn
}

output "ecr_repository_url" {
  description = "The URL of the ECR repository"
  value       = aws_ecr_repository.app_repository.repository_url
}

output "instance_role_arn" {
  description = "The ARN of the App Runner instance role"
  value       = aws_iam_role.apprunner_instance_role.arn
}

output "access_role_arn" {
  description = "The ARN of the App Runner access role"
  value       = aws_iam_role.apprunner_access_role.arn
}
