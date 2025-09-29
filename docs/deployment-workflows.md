# Dashboard Deployment Workflows

## Overview

The monolithic workflow `/.github/workflows/dashboard.yml` has been split into three focused workflows:

- `/.github/workflows/dashboard-image.yml` – build/push image and trigger App Runner (no Terraform)
- `/.github/workflows/dashboard-apply.yml` – import/update infrastructure, rebuild image, deploy
- `/.github/workflows/dashboard-recreate.yml` – destroy and recreate infrastructure, rebuild image, deploy

## When to Use Each Workflow

### Image Only
- Push code/image changes when infrastructure already exists
- Triggered automatically on pushes to `main`, or manually via **Deploy Dashboard (Image Only)**
- Fails fast if infrastructure (ECR/AppRunner) is missing

### Apply
- Update infrastructure or perform first-time provisioning
- Runs Terraform imports, targeted ECR apply, Docker build, Terraform apply, App Runner deploy
- Manual dispatch only (select branch)

### Recreate
- Tear down and rebuild everything (new App Runner URL)
- Performs extra cleanup of IAM roles, autoscaling configs, ECR repo, App Runner services
- Manual dispatch only; prints warning in logs

## Migration Notes

- The legacy workflow `dashboard.yml` should be archived/removed once the new flows are validated
- Update GitHub branch protection/deployment rules if they referenced the old workflow name
- Secrets/environment requirements remain the same: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `GCP_SERVICE_ACCOUNT_KEY`, `DB_PASSWORD`

## Manual Verification Commands

```bash
# Check available workflows
ls .github/workflows/

# Run image-only build locally (optional)
docker build -f dashboard/deploy/Dockerfile .

# Check App Runner service status
aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='data-viewer-dashboard']"
```
