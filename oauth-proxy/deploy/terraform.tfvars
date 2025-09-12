aws_region           = "us-east-1"
lambda_function_name = "oauth-proxy"
lambda_handler       = "bootstrap"
lambda_runtime       = "provided.al2"
lambda_package_path  = "./deployment-package.zip"
# lambda_environment_variables = {
#   CLIENT_ID     = "your-client-id",
#   CLIENT_SECRET = "your-client-secret"
# }
api_name      = "oauth-proxy-api"
api_route_key = "POST /oauth"