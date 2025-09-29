# Dashboard Deployment Guide

This directory contains the infrastructure-as-code setup for deploying the Data Viewer Dashboard to AWS App Runner using Terraform.

## Architecture

- **AWS App Runner**: Containerized web service hosting
- **Amazon ECR**: Container image registry
- **IAM Roles**: Proper permissions for App Runner service
- **Auto Scaling**: Automatic scaling based on traffic

## Prerequisites

1. **AWS CLI** configured with appropriate credentials
2. **Terraform** >= 1.0 installed
3. **Docker** installed for local testing
4. **GCP Service Account** credentials for Cloud SQL Proxy (JSON key file)
5. **GitHub repository** with the following secrets configured:
   - `AWS_ACCESS_KEY_ID` - AWS access key for ECR and App Runner
   - `AWS_SECRET_ACCESS_KEY` - AWS secret access key
   - `AWS_REGION` - AWS region (e.g., us-east-1)
   - `DB_PASSWORD` - Database password for Cloud SQL connection
   - `GCP_SERVICE_ACCOUNT_KEY` - Full JSON content of your GCP service account key file

## AWS IAM Permissions Required

Your AWS credentials need the following permissions:
- ECR repository management
- App Runner service management  
- IAM role/policy management
- CloudWatch logs access

## GitHub Secrets Setup

Before deploying via GitHub Actions, configure these repository secrets:

| Secret Name | Description | Example/Format |
|-------------|-------------|----------------|
| `AWS_ACCESS_KEY_ID` | AWS access key for ECR and App Runner | `AKIAIOSFODNN7EXAMPLE` |
| `AWS_SECRET_ACCESS_KEY` | AWS secret access key | `wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY` |
| `AWS_REGION` | AWS region for deployment | `us-east-1` |
| `DB_PASSWORD` | Database password for Cloud SQL | `your_secure_password` |
| `GCP_SERVICE_ACCOUNT_KEY` | Full GCP service account JSON | `{"type":"service_account",...}` |

**To add secrets:**
1. Go to your GitHub repository
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret** for each secret above

## Quick Start

### 1. Set Up GCP Credentials for Cloud SQL Proxy

The application uses Cloud SQL Proxy to connect to a GCP Cloud SQL database.

**For Local Development:**
Place your GCP service account JSON credentials file at:

```bash
# From the project root directory
mkdir -p .tmp
cp /path/to/your/gcp-service-account.json .tmp/service-client.json
```

**For GitHub Actions (CI/CD):**
Add the GCP service account JSON as a GitHub secret:

1. Go to your GitHub repository
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Name: `GCP_SERVICE_ACCOUNT_KEY`
5. Value: Paste the **entire contents** of your GCP service account JSON file
6. Click **Add secret**

Example of what the JSON content looks like:
```json
{
  "type": "service_account",
  "project_id": "your-project-id",
  "private_key_id": "...",
  "private_key": "-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n",
  "client_email": "...",
  ...
}
```

**Important**: The `.tmp/service-client.json` file will be baked into the Docker image. For security, ensure `.tmp/` is in your `.gitignore` to avoid committing credentials to version control.

### 2. Configure Database Password

You can provide the database password in several ways:

**Option 1: Environment Variable (Recommended for CI/CD)**
```bash
export TF_VAR_db_password="your_database_password"
```

**Option 2: Create a terraform.tfvars file**
```bash
cd dashboard/deploy/terraform
cat > terraform.tfvars <<EOF
db_password = "your_database_password"
EOF
```

**Option 3: Pass via command line**
```bash
terraform apply -var="db_password=your_database_password"
```

**Note**: For GitHub Actions, the `DB_PASSWORD` should be set as a repository secret.

### 3. Initial Infrastructure Deployment

```bash
# Navigate to the deploy directory
cd dashboard/deploy/terraform

# Initialize Terraform
terraform init

# Review the deployment plan (db_password should be set via one of the methods above)
terraform plan

# Deploy the infrastructure
terraform apply
```

### 4. Deploy via GitHub Actions

The deployment is automated via GitHub Actions. There are two ways to trigger deployments:

#### Automatic Deployment
- Push to `main` branch with changes in `dashboard/`, `common/`, or workflow files
- The workflow will build and push the Docker image to ECR
- App Runner will automatically deploy the new image

#### Manual Deployment
- Go to Actions tab in GitHub
- Select "Deploy Dashboard to AWS App Runner"
- Click "Run workflow"
- Choose whether to deploy infrastructure (`deploy_infrastructure: true` for first time)

## Configuration

### Environment Variables

Modify `terraform.tfvars` to customize your deployment:

```hcl
# Resource sizing
cpu    = "0.5 vCPU"    # Options: 0.25, 0.5, 1, 2, 4 vCPU
memory = "1 GB"        # Options: 0.5, 1, 2, 3, 4, 6, 8, 10, 12 GB

# Auto scaling
min_size        = 1
max_size        = 10
max_concurrency = 200

# Application configuration
environment_variables = {
  RUST_LOG     = "info"
  DATABASE_URL = "postgresql://user:pass@host:port/db"
}
```

### Application Configuration

The application expects a `config.json` file. Update the template in `dashboard/config.json` with your specific configuration needs.

### Cloud SQL Proxy Configuration

The startup script automatically starts Cloud SQL Proxy if the `CLOUD_SQL_CONNECTION_STRING` environment variable is set. The GCP credentials are baked into the Docker image at `/app/.gcp/service-client.json` and the `GOOGLE_APPLICATION_CREDENTIALS` environment variable is set in the Dockerfile.

Configure the Cloud SQL connection in your `terraform.tfvars` or `apprunner.yaml`:

```hcl
environment_variables = {
  RUST_LOG                      = "info"
  PORT                          = "8080"
  CLOUD_SQL_CONNECTION_STRING   = "PROJECT_ID:REGION:INSTANCE_NAME"  # e.g., "savvy-nimbus-306111:europe-west2:vibgo-sql"
  CLOUD_SQL_PORT                = "5433"  # Port where Cloud SQL Proxy listens
}
```

**Note**: 
- The database configuration in `config.json` should point to `localhost:5433` (or your configured `CLOUD_SQL_PORT`) since the Cloud SQL Proxy runs locally in the container.
- The `GOOGLE_APPLICATION_CREDENTIALS` environment variable is already set in the Dockerfile to `/app/.gcp/service-client.json`.

## Deployment Workflow

1. **Code Push**: Developer pushes code to main branch
2. **Build**: GitHub Actions builds Docker image
3. **Push**: Image pushed to ECR with `latest` and commit SHA tags
4. **Deploy**: App Runner automatically deploys the `latest` image
5. **Health Check**: App Runner performs health checks on the new deployment

## Monitoring and Logs

- **App Runner Console**: Monitor service health and deployments
- **CloudWatch Logs**: Application logs are automatically collected
## Troubleshooting

### Common Issues

1. **Build Failures**
   ```bash
   # Check GitHub Actions logs for build errors
   # Verify Dockerfile syntax and dependencies in deploy/Dockerfile
   ```

2. **Deployment Timeouts**
   ```bash
   # Check health check endpoint responds correctly
   ```

3. **Permission Errors**
   ```bash
   # Verify AWS credentials have required permissions
   # Check IAM role policies in Terraform
   ```

### Manual Operations

```bash
# Check App Runner service status
aws apprunner describe-service --service-arn <service-arn>

# View recent deployments  
aws apprunner list-operations --service-arn <service-arn>

# Force new deployment
aws apprunner start-deployment --service-arn <service-arn>

# View logs
aws logs describe-log-groups --log-group-name-prefix "/aws/apprunner"
```

## Cost Optimization

- App Runner charges for vCPU and memory provisioned
- Consider reducing `min_size` to 0 for development environments  
- Use smaller CPU/memory for light workloads
- Enable auto-scaling to handle traffic spikes efficiently

## Security

- ECR images are scanned for vulnerabilities
- App Runner service runs with minimal IAM permissions
- Network traffic is encrypted in transit
- Consider adding WAF for additional protection

## Cleanup

To destroy all resources:

```bash
cd dashboard/deploy
terraform destroy
```

**Note**: This will permanently delete your App Runner service and ECR repository.
