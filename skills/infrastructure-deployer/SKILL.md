---
name: infrastructure-deployer
description: Deploys infrastructure using Terraform, with validation and safety checks. Use when deploying or managing cloud infrastructure.
---

# Infrastructure Deployer

Safely deploys infrastructure changes with Terraform, including validation, planning, and approval workflows.

## Activation

Use when:
- User says "deploy infrastructure"
- User asks "apply terraform changes"
- User requests "provision resources"
- User wants to "deploy to AWS/Azure/GCP"

## Instructions

1. Validate Terraform configuration
2. Run terraform plan with detailed output
3. Present plan for review
4. Apply only after explicit approval
5. Handle errors gracefully
6. Output resource information

## Safety Checklist

Before deploying:
- ✅ Correct workspace/environment selected
- ✅ State file backed up
- ✅ Plan reviewed for unexpected changes
- ✅ No resource deletions (unless intended)
- ✅ Cost estimate reviewed (if available)
- ✅ Dependencies verified

## Deployment Workflow

### Step 1: Pre-Deployment Validation

```bash
# Check Terraform version
terraform version

# Validate configuration
terraform validate

# Format check
terraform fmt -check -recursive

# Initialize if needed
terraform init -upgrade

# Select workspace
terraform workspace select ${ENV} || terraform workspace new ${ENV}
```

### Step 2: Generate Plan

```bash
# Create plan with detailed output
terraform plan \
    -out=tfplan-$(date +%Y%m%d-%H%M%S) \
    -var-file="environments/${ENV}.tfvars" \
    -detailed-exitcode

# Show plan in human-readable format
terraform show tfplan-*

# Show resource changes summary
terraform show -json tfplan-* | jq '.resource_changes[] | {action: .change.actions, resource: .address}'
```

### Step 3: Review and Approval

Present to user:
```markdown
## Terraform Plan Summary

**Environment:** ${ENV}
**Workspace:** $(terraform workspace show)

### Changes
- **Create:** X resources
- **Update:** Y resources
- **Destroy:** Z resources

### Key Changes
1. Resource 1: Description
2. Resource 2: Description

### Estimated Cost Impact
+$X.XX/month (if available via Infracost)

**Approve deployment? (yes/no)**
```

### Step 4: Apply Changes

```bash
# Apply the plan
terraform apply tfplan-*

# Capture outputs
terraform output -json > outputs-${ENV}.json

# Tag resources (if using AWS)
# This can be done via Terraform or post-apply
```

### Step 5: Post-Deployment Verification

```bash
# Verify critical resources
terraform state list | grep -E "(instance|database|load_balancer)"

# Run smoke tests if available
./scripts/smoke-test-${ENV}.sh

# Update documentation
echo "Deployed at $(date)" >> deployments.log
```

## Environment-Specific Configurations

### Development
- Auto-approve for non-destructive changes
- Verbose logging
- Local state (or dev state bucket)

### Staging
- Require manual approval
- Run integration tests post-deploy
- Mirror production configuration

### Production
- **Always** require manual approval
- Multiple approvers for destructive changes
- Automated backup before apply
- Rollback plan ready
- Maintenance window scheduling

## Error Handling

### Common Errors and Solutions

**State Lock Error:**
```bash
# Check who has the lock
terraform force-unlock <LOCK_ID>

# Or wait for lock to release
```

**Resource Already Exists:**
```bash
# Import existing resource
terraform import aws_instance.example i-1234567890abcdef0
```

**Timeout Error:**
```bash
# Increase timeouts in resource configuration
resource "aws_instance" "example" {
  # ...
  timeouts {
    create = "60m"
    update = "60m"
    delete = "60m"
  }
}
```

## Multi-Environment Strategy

### Directory Structure
```
terraform/
├── environments/
│   ├── dev.tfvars
│   ├── staging.tfvars
│   └── prod.tfvars
├── modules/
│   ├── networking/
│   ├── compute/
│   └── database/
├── main.tf
├── variables.tf
└── outputs.tf
```

### Deployment Command Pattern
```bash
# Deploy to specific environment
terraform workspace select ${ENV}
terraform plan -var-file="environments/${ENV}.tfvars"
terraform apply -var-file="environments/${ENV}.tfvars"
```

## Cost Estimation

### Using Infracost
```bash
# Generate cost estimate
infracost breakdown --path . --terraform-var-file="environments/${ENV}.tfvars"

# Compare with existing
infracost diff --path . --terraform-var-file="environments/${ENV}.tfvars"
```

## Rollback Procedure

If deployment fails or issues detected:

```bash
# 1. Identify last good state
terraform state pull > emergency-backup.tfstate

# 2. Revert to previous plan
git checkout HEAD~1

# 3. Re-apply previous configuration
terraform apply -var-file="environments/${ENV}.tfvars"

# 4. Verify rollback
./scripts/verify-infrastructure-${ENV}.sh
```

## Best Practices

### Naming Convention
Use consistent naming for resources:
```hcl
resource "aws_instance" "web" {
  tags = {
    Name        = "leo-${var.environment}-web-${var.region}-001"
    Environment = var.environment
    ManagedBy   = "terraform"
    Project     = var.project_name
  }
}
```

### State Management
- Use remote state (S3 + DynamoDB for AWS)
- Enable state locking
- Regular state backups
- Never commit state files to git

### Security
- Never hardcode credentials
- Use AWS Secrets Manager / Azure Key Vault
- Encrypt state files
- Least privilege IAM policies
- Scan for security issues (tfsec, checkov)

## Integration with CI/CD

### GitHub Actions Example
```yaml
name: Terraform Deploy
on:
  push:
    branches: [main]
  pull_request:

jobs:
  terraform:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: hashicorp/setup-terraform@v2

      - name: Terraform Init
        run: terraform init

      - name: Terraform Plan
        run: terraform plan -out=tfplan

      - name: Terraform Apply
        if: github.ref == 'refs/heads/main'
        run: terraform apply -auto-approve tfplan
```

