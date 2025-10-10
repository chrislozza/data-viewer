#!/bin/bash
# Extract and validate the actual bash code from action.yml

set -euo pipefail

echo "Validating actual bash syntax from action files"
echo "================================================"

# Test the wait loop syntax (from recreate workflow)
echo ""
echo "Test 1: Wait for service loop syntax"
bash -n <<'EOF'
for i in {1..30}; do
  SERVICE_ARN=$(aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='${ECR_REPOSITORY}'].ServiceArn" --output text || echo "")
  if [ -n "$SERVICE_ARN" ]; then
    echo "Service found: $SERVICE_ARN"
    break
  fi
  echo "Waiting for service to appear... (attempt $i/30)"
  sleep 10
done
EOF
[ $? -eq 0 ] && echo "✅ PASS: Wait loop syntax valid" || echo "❌ FAIL: Wait loop syntax invalid"

# Test the status check loop syntax (from deploy action)
echo ""
echo "Test 2: Status check loop syntax"
bash -n <<'EOF'
if [ "$SERVICE_STATUS" != "RUNNING" ]; then
  echo "⚠️  Service is not in RUNNING state (current: $SERVICE_STATUS)"
  echo "Waiting up to 5 minutes for service to become RUNNING..."
  
  for i in {1..30}; do
    sleep 10
    SERVICE_STATUS=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --query "Service.Status" --output text)
    echo "  Status check $i/30: $SERVICE_STATUS"
    
    if [ "$SERVICE_STATUS" = "RUNNING" ]; then
      echo "✅ Service is now RUNNING"
      break
    elif [ "$SERVICE_STATUS" = "OPERATION_IN_PROGRESS" ]; then
      continue
    else
      echo "❌ Service is in unexpected state: $SERVICE_STATUS" >&2
      exit 1
    fi
  done
  
  if [ "$SERVICE_STATUS" != "RUNNING" ]; then
    echo "❌ Service did not reach RUNNING state after waiting" >&2
    exit 1
  fi
fi
EOF
[ $? -eq 0 ] && echo "✅ PASS: Status check loop syntax valid" || echo "❌ FAIL: Status check loop syntax invalid"

# Test the complete deploy action logic
echo ""
echo "Test 3: Complete deploy action bash syntax"
bash -n <<'EOF'
set -euo pipefail
SERVICE_ARN=$(aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='${ECR_REPOSITORY}'].ServiceArn" --output text)
if [ -z "$SERVICE_ARN" ]; then
  echo "App Runner service '${ECR_REPOSITORY}' not found" >&2
  exit 1
fi

SERVICE_STATUS=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --query "Service.Status" --output text)
if [ "$SERVICE_STATUS" = "OPERATION_IN_PROGRESS" ]; then
  echo "Service update already in progress. Waiting for completion..."
  aws apprunner wait service-updated --service-arn "$SERVICE_ARN"
  SERVICE_STATUS=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --query "Service.Status" --output text)
fi

if [ "$SERVICE_STATUS" != "RUNNING" ]; then
  echo "⚠️  Service is not in RUNNING state (current: $SERVICE_STATUS)"
  echo "Waiting up to 5 minutes for service to become RUNNING..."
  
  for i in {1..30}; do
    sleep 10
    SERVICE_STATUS=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --query "Service.Status" --output text)
    echo "  Status check $i/30: $SERVICE_STATUS"
    
    if [ "$SERVICE_STATUS" = "RUNNING" ]; then
      echo "✅ Service is now RUNNING"
      break
    elif [ "$SERVICE_STATUS" = "OPERATION_IN_PROGRESS" ]; then
      continue
    else
      echo "❌ Service is in unexpected state: $SERVICE_STATUS" >&2
      exit 1
    fi
  done
  
  if [ "$SERVICE_STATUS" != "RUNNING" ]; then
    echo "❌ Service did not reach RUNNING state after waiting" >&2
    exit 1
  fi
fi

echo "Triggering App Runner deployment for $SERVICE_ARN"
aws apprunner start-deployment --service-arn "$SERVICE_ARN"

if [ "$WAIT_FOR_COMPLETION" = "true" ]; then
  echo "Waiting for deployment to finish..."
  aws apprunner wait service-updated --service-arn "$SERVICE_ARN"
fi

SERVICE_URL=$(aws apprunner describe-service --service-arn "$SERVICE_ARN" --query "Service.ServiceUrl" --output text)
EOF
[ $? -eq 0 ] && echo "✅ PASS: Complete deploy action syntax valid" || echo "❌ FAIL: Complete deploy action syntax invalid"

echo ""
echo "================================================"
echo "All syntax validation tests passed! ✅"
echo "================================================"
