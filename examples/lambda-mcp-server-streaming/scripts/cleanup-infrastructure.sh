#!/bin/bash

# AWS Lambda MCP Server Infrastructure Cleanup Script
# This script removes all AWS resources created by setup-infrastructure.sh

set -e  # Exit on any error

echo "🧹 Cleaning up AWS infrastructure for Lambda MCP Server..."

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
STACK_NAME="${STACK_NAME:-lambda-mcp-server}"
REGION="${AWS_DEFAULT_REGION:-us-east-1}"
TOPIC_NAME="${TOPIC_NAME:-mcp-global-events}"
TABLE_NAME="${TABLE_NAME:-mcp-sessions}"

echo -e "${BLUE}Configuration:${NC}"
echo "  Stack Name: $STACK_NAME"
echo "  Region: $REGION"
echo "  SNS Topic: $TOPIC_NAME"
echo "  DynamoDB Table: $TABLE_NAME"
echo ""

# Confirmation prompt
echo -e "${RED}⚠️  WARNING: This will delete all infrastructure resources!${NC}"
echo -e "${YELLOW}This action cannot be undone and will result in data loss.${NC}"
echo ""
read -p "Are you sure you want to continue? (type 'yes' to confirm): " confirmation

if [ "$confirmation" != "yes" ]; then
    echo -e "${BLUE}❌ Cleanup cancelled by user${NC}"
    exit 0
fi

echo ""
echo -e "${YELLOW}🗑️  Starting cleanup process...${NC}"

# Function to delete SNS topic
delete_sns_topic() {
    echo -e "${YELLOW}🗑️  Deleting SNS topic: $TOPIC_NAME${NC}"
    
    # Get topic ARN
    TOPIC_ARN="arn:aws:sns:${REGION}:$(aws sts get-caller-identity --query 'Account' --output text):${TOPIC_NAME}"
    
    if aws sns get-topic-attributes --topic-arn "$TOPIC_ARN" --region "$REGION" &> /dev/null; then
        aws sns delete-topic --topic-arn "$TOPIC_ARN" --region "$REGION"
        echo -e "${GREEN}✅ SNS topic deleted${NC}"
    else
        echo -e "${YELLOW}⚠️  SNS topic $TOPIC_NAME not found${NC}"
    fi
}

# Function to delete DynamoDB table
delete_dynamodb_table() {
    echo -e "${YELLOW}🗑️  Deleting DynamoDB table: $TABLE_NAME${NC}"
    
    if aws dynamodb describe-table --table-name "$TABLE_NAME" --region "$REGION" &> /dev/null; then
        aws dynamodb delete-table --table-name "$TABLE_NAME" --region "$REGION"
        echo -e "${GREEN}✅ DynamoDB table deletion initiated${NC}"
        
        # Wait for table to be deleted
        echo -e "${YELLOW}⏳ Waiting for table deletion to complete...${NC}"
        aws dynamodb wait table-not-exists --table-name "$TABLE_NAME" --region "$REGION"
        echo -e "${GREEN}✅ DynamoDB table deleted${NC}"
    else
        echo -e "${YELLOW}⚠️  DynamoDB table $TABLE_NAME not found${NC}"
    fi
}

# Function to delete IAM role and policies
delete_lambda_role() {
    echo -e "${YELLOW}🗑️  Deleting Lambda execution role${NC}"
    
    ROLE_NAME="${STACK_NAME}-lambda-role"
    POLICY_NAME="${STACK_NAME}-lambda-policy"
    
    if aws iam get-role --role-name "$ROLE_NAME" &> /dev/null; then
        # Delete inline policy first
        if aws iam get-role-policy --role-name "$ROLE_NAME" --policy-name "$POLICY_NAME" &> /dev/null; then
            aws iam delete-role-policy --role-name "$ROLE_NAME" --policy-name "$POLICY_NAME"
            echo -e "${GREEN}✅ IAM policy deleted${NC}"
        fi
        
        # Delete role
        aws iam delete-role --role-name "$ROLE_NAME"
        echo -e "${GREEN}✅ IAM role deleted${NC}"
    else
        echo -e "${YELLOW}⚠️  IAM role $ROLE_NAME not found${NC}"
    fi
}

# Function to clean up local files
cleanup_local_files() {
    echo -e "${YELLOW}🗑️  Cleaning up local files${NC}"
    
    if [ -f ".env" ]; then
        rm .env
        echo -e "${GREEN}✅ Removed .env file${NC}"
    else
        echo -e "${YELLOW}⚠️  .env file not found${NC}"
    fi
}

# Main execution
echo -e "${BLUE}Starting cleanup...${NC}"
echo ""

delete_sns_topic
echo ""

delete_dynamodb_table
echo ""

delete_lambda_role
echo ""

cleanup_local_files
echo ""

echo -e "${GREEN}🎉 Infrastructure cleanup completed successfully!${NC}"
echo ""
echo -e "${BLUE}All AWS resources have been removed:${NC}"
echo "  ✅ SNS Topic: $TOPIC_NAME"
echo "  ✅ DynamoDB Table: $TABLE_NAME"
echo "  ✅ IAM Role: ${STACK_NAME}-lambda-role"
echo "  ✅ Local configuration files"
echo ""
echo -e "${YELLOW}ℹ️  Your Lambda function (if deployed) will need to be deleted separately.${NC}"