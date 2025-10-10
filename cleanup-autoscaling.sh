#!/bin/bash
set -e

echo "=========================================="
echo "Cleaning up App Runner Autoscaling Configs"
echo "=========================================="
echo ""

CONFIG_NAME="dv-dashboard-autoscale"
AWS_REGION="${AWS_REGION:-us-east-1}"

echo "🔍 Listing all versions of '$CONFIG_NAME'..."
echo ""

# Get all autoscaling configurations with this name
CONFIGS=$(aws apprunner list-auto-scaling-configurations \
  --region "$AWS_REGION" \
  --auto-scaling-configuration-name "$CONFIG_NAME" \
  --query "AutoScalingConfigurationSummaryList[].[AutoScalingConfigurationArn,Status,AutoScalingConfigurationRevision]" \
  --output text 2>/dev/null || echo "")

if [ -z "$CONFIGS" ]; then
  echo "✅ No autoscaling configurations found with name '$CONFIG_NAME'"
  exit 0
fi

echo "Found configurations:"
echo "$CONFIGS" | nl -w 2 -s ". "
echo ""

# Count total
TOTAL=$(echo "$CONFIGS" | wc -l)
echo "Total versions: $TOTAL"
echo "AWS Limit: 5 versions per configuration name"
echo ""

if [ "$TOTAL" -ge 5 ]; then
  echo "⚠️  You've hit the quota limit!"
fi

# Delete inactive configurations
echo "=========================================="
echo "Deleting INACTIVE configurations..."
echo "=========================================="
echo ""

DELETED=0
while IFS=$'\t' read -r ARN STATUS REVISION; do
  if [ "$STATUS" = "INACTIVE" ]; then
    echo "🗑️  Deleting revision $REVISION (INACTIVE)..."
    echo "   ARN: $ARN"
    
    if aws apprunner delete-auto-scaling-configuration \
      --region "$AWS_REGION" \
      --auto-scaling-configuration-arn "$ARN" 2>/dev/null; then
      echo "   ✅ Deleted successfully"
      DELETED=$((DELETED + 1))
    else
      echo "   ⚠️  Failed to delete (may be in use)"
    fi
    echo ""
  else
    echo "⏭️  Skipping revision $REVISION (status: $STATUS)"
  fi
done <<< "$CONFIGS"

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo ""
echo "Deleted: $DELETED configuration(s)"
echo ""

# Check remaining count
REMAINING=$(aws apprunner list-auto-scaling-configurations \
  --region "$AWS_REGION" \
  --auto-scaling-configuration-name "$CONFIG_NAME" \
  --query "length(AutoScalingConfigurationSummaryList)" \
  --output text 2>/dev/null || echo "0")

echo "Remaining versions: $REMAINING / 5"
echo ""

if [ "$REMAINING" -lt 5 ]; then
  echo "✅ You can now create new autoscaling configurations!"
else
  echo "⚠️  Still at quota limit. You may need to:"
  echo "   1. Delete the App Runner service first (it may be using an active config)"
  echo "   2. Then delete the active autoscaling configurations"
  echo ""
  echo "To delete all (including active):"
  echo "  aws apprunner list-auto-scaling-configurations --auto-scaling-configuration-name '$CONFIG_NAME' --query 'AutoScalingConfigurationSummaryList[].AutoScalingConfigurationArn' --output text | xargs -n1 aws apprunner delete-auto-scaling-configuration --auto-scaling-configuration-arn"
fi
