output "lambda_function_name" {
  description = "Name of the Lambda function"
  value       = aws_lambda_function.oauth_proxy.function_name
}

output "api_endpoint" {
  description = "API Gateway endpoint URL"
  value       = aws_apigatewayv2_api.oauth_api.api_endpoint
}

output "oauth_endpoint" {
  description = "OAuth token endpoint URL"
  value       = "${aws_apigatewayv2_api.oauth_api.api_endpoint}/oauth/token"
}
