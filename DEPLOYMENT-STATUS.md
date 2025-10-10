# Deployment Status & Recovery

## Current State (2025-10-10 13:59)

### ✅ What's Working
- **ECR Repository:** Exists with images
  - `latest` ✅
  - `main` ✅
  - `88f6ce5fdb938c2968cdc8f2a35cc82782beee2e` (commit SHA) ✅

### ❌ What Failed
- **App Runner Service:** CREATE_FAILED
  - Service ARN: `arn:aws:apprunner:us-east-1:899469778034:service/data-viewer-dashboard/4689b56876f64178ac5e3967f356b49b`
  - Status: CREATE_FAILED (now being deleted)
  - Reason: Service creation started before `:latest` image was pushed

### 🔧 Fixes Applied to Workflow
1. **Always push `:latest` tag** - No longer conditional on main branch
2. **Always verify `:latest` exists** - Before Terraform creates service
3. **Skip imports in recreate mode** - Prevents "already exists" errors
4. **Force cleanup AWS resources** - Removes orphaned resources after destroy
5. **Delete failed services automatically** - Init step now handles CREATE_FAILED services

## Recovery Steps

### Step 1: Wait for Service Deletion (In Progress)
```bash
# Check deletion status
aws apprunner describe-service \
  --service-arn "arn:aws:apprunner:us-east-1:899469778034:service/data-viewer-dashboard/4689b56876f64178ac5e3967f356b49b" \
  --region us-east-1 \
  --query "Service.Status" \
  --output text

# Should show: DELETE_IN_PROGRESS
# Wait a few minutes for it to complete
```

### Step 2: Verify Clean State
```bash
# No services should exist
aws apprunner list-services \
  --region us-east-1 \
  --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard']"

# Should return: []
```

### Step 3: Commit and Push Workflow Changes
```bash
git add .github/workflows/dashboard.yml
git commit -m "Fix: Always push :latest tag, cleanup failed services, skip imports in recreate"
git push origin main
```

### Step 4: Trigger Fresh Deployment
Go to GitHub Actions → Deploy Dashboard → Run workflow
- Branch: `main`
- Mode: `apply` (not recreate - infrastructure already partially exists)

The workflow will now:
1. ✅ Import existing IAM roles and ECR repo
2. ✅ Ensure ECR repository exists (already does)
3. ✅ Build and push Docker image with ALL tags (SHA, branch, **latest**)
4. ✅ Verify `:latest` tag exists in ECR
5. ✅ Run Terraform apply (will create new service)
6. ✅ Deploy to App Runner

## Alternative: Manual Terraform Apply

If you prefer to test locally:

```bash
cd dashboard/deploy/terraform

# Initialize
terraform init

# Import existing resources
terraform import aws_iam_role.apprunner_instance_role data-viewer-dashboard-apprunner-instance-role
terraform import aws_iam_role.apprunner_access_role data-viewer-dashboard-apprunner-access-role
terraform import aws_iam_role_policy.apprunner_instance_policy data-viewer-dashboard-apprunner-instance-role:data-viewer-dashboard-apprunner-instance-policy
terraform import aws_iam_role_policy_attachment.apprunner_access_role_ecr data-viewer-dashboard-apprunner-access-role/arn:aws:iam::aws:policy/service-role/AWSAppRunnerServicePolicyForECRAccess
terraform import aws_ecr_repository.app_repository data-viewer-dashboard
terraform import aws_ecr_lifecycle_policy.app_repository_policy data-viewer-dashboard

# Plan
terraform plan -var="aws_region=us-east-1"

# Apply (will create App Runner service)
terraform apply -auto-approve -var="aws_region=us-east-1"
```

## What Was Wrong (Root Cause Analysis)

### Original Problem
The workflow had a race condition in execution order:
```
1. Terraform ensure ECR (targeted apply)
2. Docker build/push
   - Pushed: commit-sha, branch-name
   - SKIPPED: latest (only pushed on main branch)
3. Terraform apply (full) - Created App Runner pointing to :latest
4. App Runner tried to pull :latest → FAILED (didn't exist)
```

### The Fix
```
1. Terraform ensure ECR (targeted apply)
2. Docker build/push
   - Pushes: commit-sha, branch-name, AND latest (always!)
3. Verify :latest exists in ECR
4. Terraform apply (full) - Creates App Runner pointing to :latest
5. App Runner pulls :latest → SUCCESS
```

## Monitoring the Next Deployment

### Check App Runner Status
```bash
# Get service ARN
SERVICE_ARN=$(aws apprunner list-services --region us-east-1 --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard'].ServiceArn" --output text)

# Watch status
watch -n 5 "aws apprunner describe-service --service-arn $SERVICE_ARN --region us-east-1 --query 'Service.Status' --output text"

# Expected progression:
# CREATE_IN_PROGRESS → RUNNING (takes 4-6 minutes)
```

### Check Deployment Logs
```bash
# List operations
aws apprunner list-operations --service-arn "$SERVICE_ARN" --region us-east-1

# Get service URL
aws apprunner describe-service --service-arn "$SERVICE_ARN" --region us-east-1 --query "Service.ServiceUrl" --output text
```

## Success Criteria

✅ App Runner service status: `RUNNING`
✅ Service URL accessible: `https://<service-url>`
✅ Health check passing: `/health` returns 200
✅ All three image tags in ECR: `latest`, `main`, `<commit-sha>`

## If It Fails Again

1. Check GitHub Actions logs for the failing step
2. Verify `:latest` tag exists in ECR before Terraform apply
3. Check App Runner operation logs: `aws apprunner list-operations`
4. Share the error message and I'll debug further

## Timeline

- **12:25 PM** - Previous deployment started
- **12:26 PM** - Failed: ECR image doesn't exist (`:latest` not pushed)
- **13:47 PM** - New deployment created service (still failed)
- **13:59 PM** - Identified root cause: `:latest` only pushed on main branch
- **14:00 PM** - Fixed workflow: Always push `:latest`
- **14:01 PM** - Deleted failed service (in progress)
- **Next** - Deploy with fixed workflow

The deployment should succeed on the next attempt with the updated workflow.
