# Simple Lambda function resource
resource "aws_lambda_function" "oauth_proxy" {
  function_name    = var.lambda_function_name
  handler          = var.lambda_handler
  runtime          = var.lambda_runtime
  filename         = var.lambda_package_path
  source_code_hash = filebase64sha256(var.lambda_package_path) # This enables code updates

  role = aws_iam_role.lambda_exec.arn

  environment {
    variables = var.lambda_environment_variables
  }

  depends_on = [aws_cloudwatch_log_group.lambda_logs]
}

# IAM role for Lambda
resource "aws_iam_role" "lambda_exec" {
  name = "oauth_proxy_lambda_role"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
    }]
  })
}

# Policy attachment for Lambda basic execution
resource "aws_iam_role_policy_attachment" "lambda_basic" {
  role       = aws_iam_role.lambda_exec.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}
