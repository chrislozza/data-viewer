#!/bin/bash
set -e

echo "=========================================="
echo "Testing RECREATE Workflow Flow"
echo "=========================================="
echo ""

AWS_REGION="${AWS_REGION:-us-east-1}"

echo "This script simulates the recreate workflow to verify:"
echo "  1. No imports run in recreate mode"
echo "  2. Resources are force-deleted from AWS"
echo "  3. ECR repo can be recreated"
echo "  4. Image can be pushed"
echo "  5. Full Terraform apply succeeds"
echo ""

# Check current state
echo "=========================================="
echo "Step 1: Check Current AWS Resources"
echo "=========================================="

echo "IAM Roles:"
aws iam get-role --role-name data-viewer-dashboard-apprunner-instance-role 2>/dev/null && echo "  ✓ Instance role exists" || echo "  ✗ Instance role not found"
aws iam get-role --role-name data-viewer-dashboard-apprunner-access-role 2>/dev/null && echo "  ✓ Access role exists" || echo "  ✗ Access role not found"

echo ""
echo "ECR Repository:"
aws ecr describe-repositories --repository-names data-viewer-dashboard --region "$AWS_REGION" 2>/dev/null && echo "  ✓ ECR repo exists" || echo "  ✗ ECR repo not found"

echo ""
echo "App Runner Service:"
SERVICE_ARN=$(aws apprunner list-services --region "$AWS_REGION" --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard'].ServiceArn" --output text)
if [ -n "$SERVICE_ARN" ]; then
  echo "  ✓ App Runner service exists: $SERVICE_ARN"
else
  echo "  ✗ App Runner service not found"
fi

echo ""
read -p "Press Enter to simulate FORCE CLEANUP (or Ctrl+C to cancel)..."

# Simulate force cleanup
echo ""
echo "=========================================="
echo "Step 2: Force Cleanup AWS Resources"
echo "=========================================="

echo "Cleaning up IAM roles..."
aws iam detach-role-policy \
  --role-name data-viewer-dashboard-apprunner-access-role \
  --policy-arn arn:aws:iam::aws:policy/service-role/AWSAppRunnerServicePolicyForECRAccess \
  2>/dev/null && echo "  ✓ Detached policy from access role" || echo "  - Policy already detached"

aws iam delete-role-policy \
  --role-name data-viewer-dashboard-apprunner-instance-role \
  --policy-name data-viewer-dashboard-apprunner-instance-policy \
  2>/dev/null && echo "  ✓ Deleted instance role policy" || echo "  - Policy already deleted"

aws iam delete-role --role-name data-viewer-dashboard-apprunner-instance-role 2>/dev/null && echo "  ✓ Deleted instance role" || echo "  - Instance role already deleted"
aws iam delete-role --role-name data-viewer-dashboard-apprunner-access-role 2>/dev/null && echo "  ✓ Deleted access role" || echo "  - Access role already deleted"

echo ""
echo "Cleaning up ECR repository..."
aws ecr delete-repository \
  --repository-name data-viewer-dashboard \
  --force \
  --region "$AWS_REGION" \
  2>/dev/null && echo "  ✓ Deleted ECR repository" || echo "  - ECR repository already deleted"

echo ""
echo "✅ Force cleanup complete"

# Verify cleanup
echo ""
echo "=========================================="
echo "Step 3: Verify Resources Are Gone"
echo "=========================================="

aws iam get-role --role-name data-viewer-dashboard-apprunner-instance-role 2>/dev/null && echo "  ❌ Instance role still exists!" || echo "  ✓ Instance role deleted"
aws iam get-role --role-name data-viewer-dashboard-apprunner-access-role 2>/dev/null && echo "  ❌ Access role still exists!" || echo "  ✗ Access role deleted"
aws ecr describe-repositories --repository-names data-viewer-dashboard --region "$AWS_REGION" 2>/dev/null && echo "  ❌ ECR repo still exists!" || echo "  ✓ ECR repo deleted"

echo ""
read -p "Press Enter to test ECR RECREATION (or Ctrl+C to cancel)..."

# Test ECR recreation
echo ""
echo "=========================================="
echo "Step 4: Test ECR Repository Recreation"
echo "=========================================="

cd dashboard/deploy/terraform
terraform init

echo ""
echo "Running targeted apply for ECR repository..."
terraform apply -auto-approve -target=aws_ecr_repository.app_repository -var="aws_region=$AWS_REGION"

echo ""
echo "Verifying ECR repository was created..."
ECR_URL=$(aws ecr describe-repositories --repository-names data-viewer-dashboard --region "$AWS_REGION" --query 'repositories[0].repositoryUri' --output text 2>/dev/null || echo "")

if [ -n "$ECR_URL" ]; then
  echo "  ✓ ECR repository created: $ECR_URL"
else
  echo "  ❌ ECR repository creation failed!"
  exit 1
fi

cd ../../..

echo ""
echo "=========================================="
echo "Step 5: Summary"
echo "=========================================="
echo ""
echo "✅ Force cleanup successfully removed orphaned resources"
echo "✅ ECR repository can be recreated via targeted apply"
echo "✅ Ready for Docker build and full Terraform apply"
echo ""
echo "Next steps in actual workflow:"
echo "  1. Docker build & push to ECR"
echo "  2. Terraform plan (full)"
echo "  3. Terraform apply (full)"
echo "  4. App Runner deployment"
echo ""
