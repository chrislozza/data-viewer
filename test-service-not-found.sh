#!/bin/bash
# Test the 'service not found' scenario for the deploy action logic

set -euo pipefail

echo "Testing 'service not found' scenario"
echo "===================================="

# Mock AWS CLI to simulate no service found
aws() {
  local cmd="$1"; shift
  case "$cmd" in
    apprunner)
      local subcmd="$1"; shift
      case "$subcmd" in
        list-services)
          # Return empty to simulate no service present
          echo ""
          ;;
        describe-service)
          echo ""
          ;;
        wait)
          echo "Waiting (mocked)"
          ;;
      esac
      ;;
  esac
}
export -f aws

# Emulate the deploy action snippet in a subshell so we can capture the exit code
ECR_REPOSITORY="data-viewer-dashboard"

set +e
OUTPUT=$(bash -euo pipefail -c '
  SERVICE_ARN=$(aws apprunner list-services --query "ServiceSummaryList[?ServiceName=='"'"${ECR_REPOSITORY}"'"'].ServiceArn" --output text)
  if [ -z "$SERVICE_ARN" ]; then
    echo "App Runner service '"${ECR_REPOSITORY}"' not found" >&2
    exit 1
  fi
' 2>&1)
STATUS=$?
set -e

echo "Captured exit: $STATUS"
echo "Captured output: $OUTPUT"

if [ $STATUS -eq 1 ] && echo "$OUTPUT" | grep -q "not found"; then
  echo "✅ PASS: Correctly fails fast when service is missing"
  exit 0
else
  echo "❌ FAIL: Unexpected behavior for missing service"
  exit 1
fi
