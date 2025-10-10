#!/bin/bash

# Verify the GitHub Actions workflow order is correct
# This analyzes the workflow file to ensure steps are in the right order

echo "=========================================="
echo "Verifying GitHub Actions Workflow Order"
echo "=========================================="
echo ""

WORKFLOW_FILE=".github/workflows/dashboard.yml"

if [ ! -f "$WORKFLOW_FILE" ]; then
    echo "❌ Workflow file not found: $WORKFLOW_FILE"
    exit 1
fi

echo "📄 Analyzing: $WORKFLOW_FILE"
echo ""

# Extract step names and their line numbers
echo "Step Order in Workflow:"
echo "----------------------------------------"

grep -n "name:" "$WORKFLOW_FILE" | grep -A1 "steps:" -A100 | grep "    - name:" | nl -v 1 -w 2 -s ". "

echo ""
echo "=========================================="
echo "Key Steps Analysis"
echo "=========================================="
echo ""

# Find line numbers for critical steps
TERRAFORM_DESTROY_LINE=$(grep -n "name: Terraform Destroy" "$WORKFLOW_FILE" | cut -d: -f1)
TERRAFORM_APPLY_LINE=$(grep -n "name: Terraform Apply" "$WORKFLOW_FILE" | cut -d: -f1)
ECR_LOGIN_LINE=$(grep -n "name: Login to Amazon ECR" "$WORKFLOW_FILE" | cut -d: -f1)
DOCKER_BUILD_LINE=$(grep -n "name: Build, tag, and push image" "$WORKFLOW_FILE" | cut -d: -f1)
APPRUNNER_DEPLOY_LINE=$(grep -n "name: Deploy to App Runner" "$WORKFLOW_FILE" | cut -d: -f1)

echo "Critical Step Line Numbers:"
echo "  Terraform Destroy:    Line $TERRAFORM_DESTROY_LINE"
echo "  Terraform Apply:      Line $TERRAFORM_APPLY_LINE"
echo "  Login to ECR:         Line $ECR_LOGIN_LINE"
echo "  Docker Build/Push:    Line $DOCKER_BUILD_LINE"
echo "  App Runner Deploy:    Line $APPRUNNER_DEPLOY_LINE"
echo ""

# Verify order
echo "=========================================="
echo "Order Verification"
echo "=========================================="
echo ""

ERRORS=0

# Check 1: Terraform Apply comes before Docker Build
if [ "$TERRAFORM_APPLY_LINE" -lt "$DOCKER_BUILD_LINE" ]; then
    echo "✅ CORRECT: Terraform Apply (line $TERRAFORM_APPLY_LINE) runs BEFORE Docker Build (line $DOCKER_BUILD_LINE)"
else
    echo "❌ ERROR: Docker Build runs before Terraform Apply!"
    ERRORS=$((ERRORS + 1))
fi

# Check 2: ECR Login comes after Terraform Apply
if [ "$ECR_LOGIN_LINE" -gt "$TERRAFORM_APPLY_LINE" ]; then
    echo "✅ CORRECT: ECR Login (line $ECR_LOGIN_LINE) runs AFTER Terraform Apply (line $TERRAFORM_APPLY_LINE)"
else
    echo "❌ ERROR: ECR Login runs before Terraform Apply!"
    ERRORS=$((ERRORS + 1))
fi

# Check 3: Docker Build comes after ECR Login
if [ "$DOCKER_BUILD_LINE" -gt "$ECR_LOGIN_LINE" ]; then
    echo "✅ CORRECT: Docker Build (line $DOCKER_BUILD_LINE) runs AFTER ECR Login (line $ECR_LOGIN_LINE)"
else
    echo "❌ ERROR: Docker Build runs before ECR Login!"
    ERRORS=$((ERRORS + 1))
fi

# Check 4: App Runner Deploy comes after Docker Build
if [ "$APPRUNNER_DEPLOY_LINE" -gt "$DOCKER_BUILD_LINE" ]; then
    echo "✅ CORRECT: App Runner Deploy (line $APPRUNNER_DEPLOY_LINE) runs AFTER Docker Build (line $DOCKER_BUILD_LINE)"
else
    echo "❌ ERROR: App Runner Deploy runs before Docker Build!"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo "=========================================="
echo "Deployment Mode Analysis"
echo "=========================================="
echo ""

# Check conditional execution
echo "Checking step conditions..."
echo ""

# Terraform steps should be conditional
if grep -A2 "name: Terraform Apply" "$WORKFLOW_FILE" | grep -q "deployment_mode != 'image-only'"; then
    echo "✅ Terraform Apply is conditional (skipped in image-only mode)"
else
    echo "⚠️  Warning: Terraform Apply condition not found"
fi

# Docker steps should always run
if grep -A2 "name: Login to Amazon ECR" "$WORKFLOW_FILE" | grep -q "if:"; then
    echo "⚠️  Warning: ECR Login has a condition (should always run)"
else
    echo "✅ ECR Login runs unconditionally (always executes)"
fi

echo ""
echo "=========================================="
echo "Recreate Mode Flow"
echo "=========================================="
echo ""

echo "In 'recreate' mode, the flow is:"
echo "  1. Terraform Destroy (line $TERRAFORM_DESTROY_LINE) - Deletes ECR repository"
echo "  2. Terraform Apply (line $TERRAFORM_APPLY_LINE) - Recreates ECR repository"
echo "  3. Login to ECR (line $ECR_LOGIN_LINE) - Authenticates to ECR"
echo "  4. Docker Build/Push (line $DOCKER_BUILD_LINE) - Pushes to ECR (now exists!)"
echo "  5. App Runner Deploy (line $APPRUNNER_DEPLOY_LINE) - Deploys the new image"
echo ""

if [ "$TERRAFORM_DESTROY_LINE" -lt "$TERRAFORM_APPLY_LINE" ] && \
   [ "$TERRAFORM_APPLY_LINE" -lt "$ECR_LOGIN_LINE" ] && \
   [ "$ECR_LOGIN_LINE" -lt "$DOCKER_BUILD_LINE" ] && \
   [ "$DOCKER_BUILD_LINE" -lt "$APPRUNNER_DEPLOY_LINE" ]; then
    echo "✅ RECREATE MODE ORDER IS CORRECT"
else
    echo "❌ RECREATE MODE ORDER IS INCORRECT"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo "=========================================="
echo "Final Result"
echo "=========================================="
echo ""

if [ $ERRORS -eq 0 ]; then
    echo "✅ ✅ ✅ WORKFLOW ORDER IS CORRECT ✅ ✅ ✅"
    echo ""
    echo "The deployment will work because:"
    echo "  • Terraform creates ECR repository FIRST"
    echo "  • Docker push happens AFTER ECR exists"
    echo "  • App Runner deployment happens LAST"
    echo ""
    echo "The bug is FIXED! 🎉"
    exit 0
else
    echo "❌ WORKFLOW HAS $ERRORS ERROR(S)"
    echo ""
    echo "The deployment will FAIL!"
    exit 1
fi
