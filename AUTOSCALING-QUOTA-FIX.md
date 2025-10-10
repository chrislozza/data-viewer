# App Runner Autoscaling Quota Issue - Fixed

## Problem

AWS App Runner has a quota of **5 versions per autoscaling configuration name**. Each time you run `terraform apply` or `recreate`, it creates a new version. After 5 deployments, you hit the quota:

```
Error: creating App Runner AutoScaling Configuration Version (dv-dashboard-autoscale): 
ServiceQuotaExceededException: you exceeded your account's revision quota: 5
```

## Root Cause

Terraform creates a new **version** of the autoscaling configuration each time, but AWS doesn't automatically delete old inactive versions. They accumulate until you hit the 5-version limit.

## Solution

### Immediate Fix (Run Now)

Clean up old autoscaling configuration versions:

```bash
./force-cleanup-autoscaling.sh
```

This script:
- Lists all versions of `dv-dashboard-autoscale`
- Keeps the newest version
- Deletes older versions (revisions 1-4)
- Frees up quota space

**Result:** Deleted 4 versions, now at 1/5 quota ✅

### Long-term Fix (Already Applied)

Updated the GitHub Actions workflow to automatically clean up old autoscaling configurations before running `recreate` mode:

**File:** `.github/workflows/dashboard.yml`
**Step:** "Cleanup old autoscaling configurations (if recreating)" (line 106)

This step:
1. Runs before Terraform Destroy
2. Lists all autoscaling config versions
3. Keeps only the newest version
4. Deletes older versions to prevent quota issues

## Verification

After cleanup, you should see:
```
Remaining versions: 1 / 5
✅ Success! You can now run terraform apply
```

## Alternative Solutions

If you continue to hit quota issues:

### Option 1: Use a different configuration name
Edit `dashboard/deploy/terraform/main.tf` line 187:
```hcl
auto_scaling_configuration_name = "dv-dashboard-autoscale-v2"  # Change name
```

### Option 2: Manually delete all versions
```bash
aws apprunner list-auto-scaling-configurations \
  --auto-scaling-configuration-name 'dv-dashboard-autoscale' \
  --query 'AutoScalingConfigurationSummaryList[].AutoScalingConfigurationArn' \
  --output text | \
  xargs -n1 aws apprunner delete-auto-scaling-configuration --auto-scaling-configuration-arn
```

### Option 3: Delete App Runner service first
If configs are "in use" and can't be deleted:
```bash
# Find service ARN
aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard'].ServiceArn" --output text

# Delete service
aws apprunner delete-service --service-arn <SERVICE_ARN>

# Then delete autoscaling configs
./force-cleanup-autoscaling.sh
```

## AWS Quotas

- **Autoscaling configuration versions per name:** 5 (hard limit)
- **Total autoscaling configurations per account:** 100
- **App Runner services per region:** 25

## Next Steps

1. ✅ Cleanup completed - quota freed
2. ✅ Workflow updated - automatic cleanup on recreate
3. 🚀 You can now retry your deployment with `recreate` mode

The workflow will now automatically manage autoscaling configuration versions to prevent hitting the quota limit in future deployments.
