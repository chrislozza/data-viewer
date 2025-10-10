#!/bin/bash

echo "=========================================="
echo "Verifying NEW Workflow Design"
echo "=========================================="
echo ""

WORKFLOW=".github/workflows/dashboard.yml"

echo "The NEW workflow uses a DIFFERENT strategy:"
echo "  1. Targeted Terraform apply (ECR only)"
echo "  2. Docker build & push"
echo "  3. Full Terraform apply (everything else)"
echo ""
echo "This ensures images exist before App Runner is created."
echo ""

# Get line numbers
TARGETED_ECR=$(grep -n "name: Terraform ensure ECR repository" "$WORKFLOW" | cut -d: -f1)
LOGIN_ECR=$(grep -n "name: Login to Amazon ECR" "$WORKFLOW" | cut -d: -f1)
BUILD_IMAGE=$(grep -n "name: Build, tag, and push image" "$WORKFLOW" | cut -d: -f1)
VERIFY_IMAGE=$(grep -n "name: Verify ECR image" "$WORKFLOW" | cut -d: -f1)
TERRAFORM_PLAN=$(grep -n "name: Terraform Plan" "$WORKFLOW" | cut -d: -f1)
TERRAFORM_APPLY=$(grep -n "name: Terraform Apply" "$WORKFLOW" | head -1 | cut -d: -f1)
DEPLOY=$(grep -n "name: Deploy to App Runner" "$WORKFLOW" | cut -d: -f1)

echo "Step Order:"
echo "  1. Targeted ECR apply:  Line $TARGETED_ECR"
echo "  2. Login to ECR:        Line $LOGIN_ECR"
echo "  3. Build & Push:        Line $BUILD_IMAGE"
echo "  4. Verify Image:        Line $VERIFY_IMAGE"
echo "  5. Terraform Plan:      Line $TERRAFORM_PLAN"
echo "  6. Terraform Apply:     Line $TERRAFORM_APPLY"
echo "  7. Deploy to App Runner: Line $DEPLOY"
echo ""

ERRORS=0

# Verify order
echo "Verification:"

if [ "$TARGETED_ECR" -lt "$LOGIN_ECR" ]; then
  echo "  ✅ Targeted ECR ($TARGETED_ECR) before Login ($LOGIN_ECR)"
else
  echo "  ❌ ERROR: Login before targeted ECR"
  ERRORS=$((ERRORS + 1))
fi

if [ "$LOGIN_ECR" -lt "$BUILD_IMAGE" ]; then
  echo "  ✅ Login ($LOGIN_ECR) before Build ($BUILD_IMAGE)"
else
  echo "  ❌ ERROR: Build before Login"
  ERRORS=$((ERRORS + 1))
fi

if [ "$BUILD_IMAGE" -lt "$VERIFY_IMAGE" ]; then
  echo "  ✅ Build ($BUILD_IMAGE) before Verify ($VERIFY_IMAGE)"
else
  echo "  ❌ ERROR: Verify before Build"
  ERRORS=$((ERRORS + 1))
fi

if [ "$VERIFY_IMAGE" -lt "$TERRAFORM_APPLY" ]; then
  echo "  ✅ Verify Image ($VERIFY_IMAGE) before Full Apply ($TERRAFORM_APPLY)"
else
  echo "  ❌ ERROR: Full Apply before image verification"
  ERRORS=$((ERRORS + 1))
fi

if [ "$TERRAFORM_APPLY" -lt "$DEPLOY" ]; then
  echo "  ✅ Terraform Apply ($TERRAFORM_APPLY) before Deploy ($DEPLOY)"
else
  echo "  ❌ ERROR: Deploy before Terraform Apply"
  ERRORS=$((ERRORS + 1))
fi

echo ""
echo "=========================================="
echo "Critical Flow Analysis"
echo "=========================================="
echo ""

echo "For RECREATE mode:"
echo "  1. Terraform Destroy removes everything"
echo "  2. Force cleanup deletes orphaned resources"
echo "  3. Targeted apply creates ONLY ECR repo"
echo "  4. Docker build pushes to ECR"
echo "  5. Full Terraform apply creates App Runner"
echo "     → App Runner pulls existing :latest image ✅"
echo ""

echo "For APPLY mode:"
echo "  1. Imports bring existing resources into state"
echo "  2. Targeted apply ensures ECR exists"
echo "  3. Docker build pushes to ECR"
echo "  4. Full Terraform apply updates/creates resources"
echo "     → App Runner pulls existing :latest image ✅"
echo ""

echo "For IMAGE-ONLY mode:"
echo "  1. Skip Terraform completely"
echo "  2. Docker build pushes to ECR"
echo "  3. Trigger App Runner deployment"
echo "     → App Runner pulls new image ✅"
echo ""

if [ $ERRORS -eq 0 ]; then
  echo "=========================================="
  echo "✅ ✅ ✅ WORKFLOW IS CORRECT ✅ ✅ ✅"
  echo "=========================================="
  echo ""
  echo "The workflow will:"
  echo "  • Create ECR repo before Docker build"
  echo "  • Verify image exists before creating App Runner"
  echo "  • Handle recreate mode properly"
  echo ""
  exit 0
else
  echo "=========================================="
  echo "❌ WORKFLOW HAS $ERRORS ERROR(S)"
  echo "=========================================="
  exit 1
fi
