# API Gateway HTTP API
resource "aws_apigatewayv2_api" "oauth_api" {
  name          = "oauth-proxy-api"
  protocol_type = "HTTP"

  cors_configuration {
    allow_origins = ["*"] # In production, restrict this to specific domains
    allow_methods = ["GET", "POST", "OPTIONS"]
    allow_headers = ["content-type", "authorization"]
    max_age       = 300
  }
}

# Default stage for the API
resource "aws_apigatewayv2_stage" "default" {
  api_id      = aws_apigatewayv2_api.oauth_api.id
  name        = "$default"
  auto_deploy = true

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api_logs.arn
    format = jsonencode({
      requestId      = "$context.requestId"
      ip             = "$context.identity.sourceIp"
      requestTime    = "$context.requestTime"
      httpMethod     = "$context.httpMethod"
      routeKey       = "$context.routeKey"
      status         = "$context.status"
      protocol       = "$context.protocol"
      responseLength = "$context.responseLength"
      errorMessage   = "$context.error.message"
    })
  }
}

# Create routes for your endpoints
resource "aws_apigatewayv2_route" "oauth_route" {
  api_id           = aws_apigatewayv2_api.oauth_api.id
  route_key        = var.api_route_key
  target           = "integrations/${aws_apigatewayv2_integration.oauth_integration.id}"
  api_key_required = true
}

# Create integration with your Lambda
resource "aws_apigatewayv2_integration" "oauth_integration" {
  api_id             = aws_apigatewayv2_api.oauth_api.id
  integration_type   = "AWS_PROXY"
  integration_uri    = aws_lambda_function.oauth_proxy.invoke_arn
  integration_method = "POST"
}

# Permission for API Gateway to invoke Lambda
resource "aws_lambda_permission" "api_gw" {
  statement_id  = "AllowExecutionFromAPIGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.oauth_proxy.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.oauth_api.execution_arn}/*/*"
}

# CloudWatch log group for API Gateway
resource "aws_cloudwatch_log_group" "api_logs" {
  name              = "/aws/apigateway/${aws_apigatewayv2_api.oauth_api.name}"
  retention_in_days = 30
}
