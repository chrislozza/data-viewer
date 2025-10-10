#!/bin/bash
# Test script for the wait-for-service logic in recreate workflow

set -euo pipefail

echo "Testing Wait-for-Service Logic"
echo "==============================="

# Mock AWS CLI for testing
aws() {
  local cmd="$1"
  shift
  
  case "$cmd" in
    apprunner)
      local subcmd="$1"
      shift
      case "$subcmd" in
        list-services)
          # Simulate service appearing after a few calls
          if [ ! -f /tmp/service_exists ]; then
            CALL_COUNT=$(cat /tmp/service_call_count 2>/dev/null || echo "0")
            CALL_COUNT=$((CALL_COUNT + 1))
            echo "$CALL_COUNT" > /tmp/service_call_count
            
            if [ "$CALL_COUNT" -ge 3 ]; then
              touch /tmp/service_exists
              echo "arn:aws:apprunner:us-east-1:123456789012:service/data-viewer-dashboard/abc123"
            else
              echo ""
            fi
          else
            echo "arn:aws:apprunner:us-east-1:123456789012:service/data-viewer-dashboard/abc123"
          fi
          ;;
        describe-service)
          echo "RUNNING"
          ;;
        wait)
          echo "Waiting (mocked)..."
          return 0
          ;;
      esac
      ;;
  esac
}

export -f aws

# Clean up from previous runs
rm -f /tmp/service_exists /tmp/service_call_count

echo ""
echo "Test: Service appears after Terraform recreate"
echo "----------------------------------------------"

ECR_REPOSITORY="data-viewer-dashboard"

echo "Waiting for App Runner service to be created and ready..."

# Wait for service to exist
for i in {1..10}; do  # Using 10 instead of 30 for testing
  SERVICE_ARN=$(aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='${ECR_REPOSITORY}'].ServiceArn" --output text || echo "")
  if [ -n "$SERVICE_ARN" ]; then
    echo "✅ Service found: $SERVICE_ARN"
    break
  fi
  echo "Waiting for service to appear... (attempt $i/10)"
  sleep 1  # Using 1 second instead of 10 for testing
done

if [ -z "$SERVICE_ARN" ]; then
  echo "❌ FAIL: Service not found after waiting"
  exit 1
fi

# Wait for service to be running
echo "Waiting for service to reach RUNNING state..."
aws apprunner wait service-updated --service-arn "$SERVICE_ARN" || true

SERVICE_STATUS=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --query "Service.Status" --output text)
echo "Service status: $SERVICE_STATUS"

if [ "$SERVICE_STATUS" = "RUNNING" ]; then
  echo "✅ PASS: Service is ready"
else
  echo "⚠️  Service status is $SERVICE_STATUS, proceeding anyway"
fi

# Clean up
rm -f /tmp/service_exists /tmp/service_call_count

echo ""
echo "==============================="
echo "All tests passed! ✅"
echo "==============================="
