#!/bin/bash
set -e

echo "=========================================="
echo "Force Cleanup App Runner Autoscaling"
echo "=========================================="
echo ""

CONFIG_NAME="dv-dashboard-autoscale"
AWS_REGION="${AWS_REGION:-us-east-1}"

# First, check if any App Runner services are using these configs
echo "🔍 Checking for App Runner services..."
SERVICES=$(aws apprunner list-services --region "$AWS_REGION" --query "ServiceSummaryList[].ServiceArn" --output text 2>/dev/null || echo "")

if [ -n "$SERVICES" ]; then
  echo "Found App Runner services:"
  for SERVICE_ARN in $SERVICES; do
    SERVICE_NAME=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --region "$AWS_REGION" --query "Service.ServiceName" --output text)
    AUTOSCALE_ARN=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --region "$AWS_REGION" --query "Service.AutoScalingConfigurationSummary.AutoScalingConfigurationArn" --output text)
    echo "  - $SERVICE_NAME"
    echo "    Using: $AUTOSCALE_ARN"
  done
  echo ""
fi

# Get all autoscaling configurations
echo "🔍 Listing all versions of '$CONFIG_NAME'..."
ARNS=$(aws apprunner list-auto-scaling-configurations \
  --region "$AWS_REGION" \
  --auto-scaling-configuration-name "$CONFIG_NAME" \
  --query "AutoScalingConfigurationSummaryList[].AutoScalingConfigurationArn" \
  --output text 2>/dev/null || echo "")

if [ -z "$ARNS" ]; then
  echo "✅ No autoscaling configurations found"
  exit 0
fi

# Convert to array
ARN_ARRAY=($ARNS)
TOTAL=${#ARN_ARRAY[@]}

echo "Found $TOTAL versions"
echo ""

# Keep only the most recent one, delete the rest
echo "Strategy: Keep the newest version, delete older versions"
echo ""

# Sort by revision number (newest last)
SORTED_ARNS=$(aws apprunner list-auto-scaling-configurations \
  --region "$AWS_REGION" \
  --auto-scaling-configuration-name "$CONFIG_NAME" \
  --query "AutoScalingConfigurationSummaryList | sort_by(@, &AutoScalingConfigurationRevision)[].AutoScalingConfigurationArn" \
  --output text)

SORTED_ARRAY=($SORTED_ARNS)
KEEP_NEWEST=${SORTED_ARRAY[-1]}

echo "Will keep: $KEEP_NEWEST (newest)"
echo ""
echo "Will delete older versions:"

DELETED=0
FAILED=0

for ARN in "${SORTED_ARRAY[@]}"; do
  if [ "$ARN" = "$KEEP_NEWEST" ]; then
    continue
  fi
  
  REVISION=$(echo "$ARN" | grep -oP '/\d+/' | tail -1 | tr -d '/')
  echo "  🗑️  Deleting revision $REVISION..."
  
  if aws apprunner delete-auto-scaling-configuration \
    --region "$AWS_REGION" \
    --auto-scaling-configuration-arn "$ARN" 2>&1; then
    echo "     ✅ Deleted"
    DELETED=$((DELETED + 1))
  else
    echo "     ⚠️  Failed (may be in use by a service)"
    FAILED=$((FAILED + 1))
  fi
done

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo "Deleted: $DELETED"
echo "Failed: $FAILED"
echo ""

# Check remaining
REMAINING=$(aws apprunner list-auto-scaling-configurations \
  --region "$AWS_REGION" \
  --auto-scaling-configuration-name "$CONFIG_NAME" \
  --query "length(AutoScalingConfigurationSummaryList)" \
  --output text 2>/dev/null || echo "0")

echo "Remaining versions: $REMAINING / 5"

if [ "$REMAINING" -lt 5 ]; then
  echo "✅ Success! You can now run terraform apply"
else
  echo ""
  echo "⚠️  Still at quota. The configs may be in use by App Runner services."
  echo ""
  echo "Option 1: Delete the App Runner service first, then re-run this script"
  echo "  aws apprunner delete-service --service-arn <SERVICE_ARN>"
  echo ""
  echo "Option 2: Use a different autoscaling configuration name in main.tf"
  echo "  Change 'dv-dashboard-autoscale' to 'dv-dashboard-autoscale-v2'"
fi
