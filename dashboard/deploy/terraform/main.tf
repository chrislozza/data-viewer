terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# IAM Role for App Runner
resource "aws_iam_role" "apprunner_instance_role" {
  name = "${var.app_name}-apprunner-instance-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "tasks.apprunner.amazonaws.com"
        }
      }
    ]
  })
}

# IAM Role for App Runner Access (build and deploy)
resource "aws_iam_role" "apprunner_access_role" {
  name = "${var.app_name}-apprunner-access-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "build.apprunner.amazonaws.com"
        }
      }
    ]
  })
}

# Attach ECR access policy to access role
resource "aws_iam_role_policy_attachment" "apprunner_access_role_ecr" {
  role       = aws_iam_role.apprunner_access_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSAppRunnerServicePolicyForECRAccess"
}

# Custom policy for instance role (add any additional permissions your app needs)
resource "aws_iam_role_policy" "apprunner_instance_policy" {
  name = "${var.app_name}-apprunner-instance-policy"
  role = aws_iam_role.apprunner_instance_role.name

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "ssm:GetParameter",
          "ssm:GetParameters",
          "ssm:GetParametersByPath"
        ]
        Resource = [
          "arn:aws:ssm:${var.aws_region}:*:parameter/${var.app_name}/*",
          "arn:aws:ssm:${var.aws_region}:*:parameter/shared/*"
        ]
      },
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          "arn:aws:secretsmanager:${var.aws_region}:*:secret:${var.app_name}/*"
        ]
      }
    ]
  })
}

# ECR Repository for the application
resource "aws_ecr_repository" "app_repository" {
  name                 = var.app_name
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  image_scanning_configuration {
    scan_on_push = true
  }

}

# ECR Lifecycle Policy
resource "aws_ecr_lifecycle_policy" "app_repository_policy" {
  repository = aws_ecr_repository.app_repository.name

  policy = jsonencode({
    rules = [
      {
        rulePriority = 1
        description  = "Keep last 10 images"
        selection = {
          tagStatus     = "tagged"
          tagPrefixList = ["v"]
          countType     = "imageCountMoreThan"
          countNumber   = 10
        }
        action = {
          type = "expire"
        }
      }
    ]
  })
}

# App Runner Service
resource "aws_apprunner_service" "dashboard_service" {
  service_name = var.app_name

  source_configuration {
    image_repository {
      image_identifier      = "${aws_ecr_repository.app_repository.repository_url}:latest"
      image_configuration {
        port = var.app_port
        runtime_environment_variables = merge(
          var.environment_variables,
          var.db_password != "" ? {
            DB_PASSWORD = var.db_password
          } : {}
        )
        start_command = var.start_command
      }
      image_repository_type = "ECR"
    }
    auto_deployments_enabled = var.auto_deployments_enabled
    
    authentication_configuration {
      access_role_arn = aws_iam_role.apprunner_access_role.arn
    }
  }

  instance_configuration {
    cpu    = var.cpu
    memory = var.memory
    instance_role_arn = aws_iam_role.apprunner_instance_role.arn
  }

  health_check_configuration {
    healthy_threshold   = 1
    interval            = 10
    path               = var.health_check_path
    protocol           = "HTTP"
    timeout            = 5
    unhealthy_threshold = 5
  }

  auto_scaling_configuration_arn = aws_apprunner_auto_scaling_configuration_version.dashboard_autoscaling.arn

  depends_on = [
    aws_iam_role_policy_attachment.apprunner_access_role_ecr
  ]

  tags = var.tags
}

# Auto Scaling Configuration
resource "aws_apprunner_auto_scaling_configuration_version" "dashboard_autoscaling" {
  auto_scaling_configuration_name = "dv-dashboard-autoscale"
  
  max_concurrency = var.max_concurrency
  max_size        = var.max_size
  min_size        = var.min_size

  tags = var.tags
}
