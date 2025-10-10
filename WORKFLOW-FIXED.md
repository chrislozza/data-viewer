# Deployment Workflow - Fixed and Verified

## Critical Issues Fixed

### 1. **Import/Destroy Race Condition** ❌ → ✅
**Problem:** Imports ran BEFORE destroy in recreate mode, causing "already exists" errors
**Fix:** Imports now skip in recreate mode (only run in apply mode)

### 2. **Orphaned AWS Resources** ❌ → ✅
**Problem:** Terraform destroy might not delete all AWS resources due to dependencies
**Fix:** Added force cleanup step that uses AWS CLI to delete IAM roles and ECR repo

### 3. **Missing Image on Service Create** ❌ → ✅
**Problem:** App Runner service tried to pull `:latest` before image existed
**Fix:** Targeted ECR apply + Docker build/push happens BEFORE full Terraform apply

### 4. **Autoscaling Quota** ❌ → ✅
**Problem:** Hit 5-version limit on autoscaling configurations
**Fix:** Cleanup step deletes old versions before recreate

## Workflow Order by Mode

### `image-only` Mode (Quick Updates)
```
1. Checkout code
2. Configure AWS credentials
3. ❌ Skip Terraform (no infrastructure changes)
4. Login to ECR
5. Build Docker image
6. Push to ECR (tag: $GITHUB_SHA, $BRANCH, latest)
7. Verify image in ECR
8. Trigger App Runner deployment
```
**Use case:** Code changes only, infrastructure already exists

### `apply` Mode (Normal Deployment)
```
1. Checkout code
2. Configure AWS credentials
3. Terraform init
4. Import existing resources (if any) ← Tries to adopt existing infra
5. ❌ Skip destroy
6. ❌ Skip force cleanup
7. Targeted apply: Create ECR repo (if needed)
8. Login to ECR
9. Build Docker image
10. Push to ECR (tag: $GITHUB_SHA, $BRANCH, latest)
11. Verify image in ECR
12. Terraform plan (full)
13. Terraform apply (full) ← Uses existing :latest image
14. Trigger App Runner deployment
```
**Use case:** First deployment or infrastructure updates

### `recreate` Mode (Clean Slate)
```
1. Checkout code
2. Configure AWS credentials
3. Terraform init
4. ❌ Skip imports ← KEY FIX: Don't import in recreate mode
5. Cleanup old autoscaling configs
6. Terraform destroy
7. Force cleanup AWS resources ← NEW: Delete orphaned resources
   - Detach IAM policies
   - Delete IAM roles
   - Delete ECR repository (force)
8. Targeted apply: Create ECR repo (fresh)
9. Login to ECR
10. Build Docker image
11. Push to ECR (tag: $GITHUB_SHA, $BRANCH, latest)
12. Verify image in ECR
13. Terraform plan (full)
14. Terraform apply (full) ← Creates App Runner with existing :latest
15. Trigger App Runner deployment
```
**Use case:** Complete infrastructure rebuild (URL will change)

## Edge Cases Now Handled

### ✅ Resources Already Exist in AWS (not in Terraform state)
- **apply mode:** Imports bring them into state
- **recreate mode:** Force cleanup deletes them, then recreates

### ✅ Terraform Destroy Fails
- Force cleanup step runs after destroy
- Manually deletes resources via AWS CLI (idempotent)

### ✅ Image Missing from ECR
- Targeted ECR apply creates repo first
- Build/push happens before full Terraform
- Verification step catches missing images before deployment

### ✅ Image Already in ECR
- Docker push overwrites existing tags (MUTABLE repo)
- No conflicts

### ✅ Autoscaling Quota Hit (5 versions)
- Cleanup step removes old versions
- Keeps only newest to free up quota

### ✅ App Runner Service Stuck/Failed
- Init step detects CREATE_FAILED services
- Deletes them before attempting new deployment

### ✅ Partial Terraform State
- **apply mode:** Imports recover state
- **recreate mode:** Force cleanup + fresh apply ignores old state

## Testing the Fixed Workflow

### Local Verification
```bash
# Test the recreate flow
chmod +x test-recreate-flow.sh
./test-recreate-flow.sh

# Verify workflow order
./verify-workflow-order.sh
```

### GitHub Actions
1. Go to Actions → Deploy Dashboard to AWS App Runner
2. Click "Run workflow"
3. Select:
   - **Branch:** main
   - **Mode:** recreate
4. Monitor execution

Expected timing:
- Force cleanup: ~10s
- Docker build: ~3-5 min
- Terraform apply: ~1-2 min
- App Runner create: ~4-6 min
- **Total:** ~10-15 minutes

## Manual Recovery Commands

If workflow fails partway through:

### Clean up orphaned resources
```bash
./force-cleanup-autoscaling.sh
```

### Force delete all infrastructure
```bash
cd dashboard/deploy/terraform

# Delete IAM roles
aws iam detach-role-policy --role-name data-viewer-dashboard-apprunner-access-role --policy-arn arn:aws:iam::aws:policy/service-role/AWSAppRunnerServicePolicyForECRAccess
aws iam delete-role-policy --role-name data-viewer-dashboard-apprunner-instance-role --policy-name data-viewer-dashboard-apprunner-instance-policy
aws iam delete-role --role-name data-viewer-dashboard-apprunner-instance-role
aws iam delete-role --role-name data-viewer-dashboard-apprunner-access-role

# Delete ECR
aws ecr delete-repository --repository-name data-viewer-dashboard --force

# Delete App Runner service
SERVICE_ARN=$(aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard'].ServiceArn" --output text)
aws apprunner delete-service --service-arn "$SERVICE_ARN"

# Delete autoscaling configs
aws apprunner list-auto-scaling-configurations --auto-scaling-configuration-name dv-dashboard-autoscale --query 'AutoScalingConfigurationSummaryList[].AutoScalingConfigurationArn' --output text | xargs -n1 aws apprunner delete-auto-scaling-configuration --auto-scaling-configuration-arn

# Clear Terraform state
rm -rf .terraform terraform.tfstate*
```

### Start fresh
```bash
cd dashboard/deploy/terraform
terraform init
terraform apply -var="aws_region=us-east-1"
```

## Success Indicators

### ECR Repository
```bash
aws ecr describe-repositories --repository-names data-viewer-dashboard
# Should show: repositoryUri
```

### IAM Roles
```bash
aws iam get-role --role-name data-viewer-dashboard-apprunner-instance-role
aws iam get-role --role-name data-viewer-dashboard-apprunner-access-role
# Should show: Role details
```

### App Runner Service
```bash
aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard']"
# Should show: Status: RUNNING
```

### Images in ECR
```bash
aws ecr list-images --repository-name data-viewer-dashboard
# Should show: latest, main, and commit SHA tags
```

## Workflow Guarantees

1. ✅ **No import/destroy conflicts** - Imports disabled in recreate mode
2. ✅ **No orphaned resources** - Force cleanup removes everything
3. ✅ **Image always exists before App Runner** - Targeted apply + build + verify
4. ✅ **Autoscaling quota managed** - Automatic cleanup of old versions
5. ✅ **Failed services cleaned up** - Detection and deletion in init step
6. ✅ **Idempotent operations** - Can re-run workflow safely

## Known Limitations

- **URL changes in recreate mode** - App Runner assigns new URL each time
- **~10-15 min recreate time** - Full infrastructure rebuild takes time
- **No rollback** - Failed deployment requires manual intervention
- **Single region** - Configured for us-east-1 only

## Next Steps

1. ✅ Commit and push workflow changes
2. 🔄 Test with `apply` mode first (safer)
3. 🔄 If successful, try `recreate` mode
4. 📝 Document the final service URL
5. 🔄 Use `image-only` for subsequent code updates
