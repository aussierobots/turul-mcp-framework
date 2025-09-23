#!/bin/bash

# AWS Lambda MCP Server Infrastructure Setup Script
# This script creates the required AWS resources for the MCP server

set -e  # Exit on any error

echo "🚀 Setting up AWS infrastructure for Lambda MCP Server..."

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
echo "  SNS Topic: $TOPIC_NAME (optional)"
echo "  DynamoDB Table: $TABLE_NAME"
echo ""

# Check AWS CLI
if ! command -v aws &> /dev/null; then
    echo -e "${RED}❌ AWS CLI not found. Please install AWS CLI first.${NC}"
    exit 1
fi

# Check AWS credentials
if ! aws sts get-caller-identity &> /dev/null; then
    echo -e "${RED}❌ AWS credentials not configured. Please run 'aws configure' first.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ AWS CLI and credentials configured${NC}"

# Function to create SNS topic (optional)
create_sns_topic() {
    echo -e "${YELLOW}📝 Creating SNS topic (optional): $TOPIC_NAME${NC}"
    echo -e "${BLUE}ℹ️  SNS topic is optional - server works without it using internal tokio broadcast${NC}"

    # Check if topic already exists
    TOPIC_ARN=""
    if aws sns get-topic-attributes --topic-arn "arn:aws:sns:${REGION}:$(aws sts get-caller-identity --query 'Account' --output text):${TOPIC_NAME}" --region "$REGION" &> /dev/null; then
        echo -e "${YELLOW}⚠️  SNS topic $TOPIC_NAME already exists${NC}"
        TOPIC_ARN="arn:aws:sns:${REGION}:$(aws sts get-caller-identity --query 'Account' --output text):${TOPIC_NAME}"
    else
        # Ask if user wants to create SNS topic
        echo ""
        echo -e "${BLUE}❓ Do you want to create an SNS topic for global event publishing?${NC}"
        echo "   This enables external systems to publish global events to all MCP sessions."
        echo "   You can skip this and still use the internal tokio broadcast system."
        echo ""
        read -p "Create SNS topic? (y/N): " create_sns

        if [[ "$create_sns" =~ ^[Yy]$ ]]; then
            # Create topic
            TOPIC_ARN=$(aws sns create-topic \
                --name "$TOPIC_NAME" \
                --region "$REGION" \
                --attributes '{
                    "DisplayName": "MCP Global Events",
                    "DeliveryPolicy": "{\"http\":{\"defaultHealthyRetryPolicy\":{\"numRetries\":3,\"numMaxDelayRetries\":0,\"minDelayTarget\":20,\"maxDelayTarget\":20,\"numMinDelayRetries\":0,\"numNoDelayRetries\":0,\"backoffFunction\":\"linear\"},\"disableSubscriptionOverrides\":false}}"
                }' \
                --tags Key=Project,Value=Lambda-MCP-Server \
                --query 'TopicArn' \
                --output text)

            echo -e "${GREEN}✅ SNS topic created: $TOPIC_ARN${NC}"
        else
            echo -e "${BLUE}⏭️  Skipping SNS topic creation${NC}"
        fi
    fi

    if [ ! -z "$TOPIC_ARN" ]; then
        echo "  Topic ARN: $TOPIC_ARN"
    fi
}

# Function to create DynamoDB table
create_dynamodb_table() {
    echo -e "${YELLOW}📝 Creating DynamoDB table: $TABLE_NAME${NC}"

    # Check if table already exists
    if aws dynamodb describe-table --table-name "$TABLE_NAME" --region "$REGION" &> /dev/null; then
        echo -e "${YELLOW}⚠️  DynamoDB table $TABLE_NAME already exists${NC}"
    else
        # Create table with MCP session schema
        aws dynamodb create-table \
            --table-name "$TABLE_NAME" \
            --region "$REGION" \
            --attribute-definitions '[
                {
                    "AttributeName": "session_id",
                    "AttributeType": "S"
                }
            ]' \
            --key-schema '[
                {
                    "AttributeName": "session_id",
                    "KeyType": "HASH"
                }
            ]' \
            --billing-mode PAY_PER_REQUEST \
            --stream-specification StreamEnabled=true,StreamViewType=NEW_AND_OLD_IMAGES \
            --tags '[
                {
                    "Key": "Project",
                    "Value": "Lambda-MCP-Server"
                },
                {
                    "Key": "Purpose",
                    "Value": "Session-Management"
                }
            ]'

        echo -e "${GREEN}✅ DynamoDB table created: $TABLE_NAME${NC}"

        # Wait for table to be active
        echo -e "${YELLOW}⏳ Waiting for table to become active...${NC}"
        aws dynamodb wait table-exists --table-name "$TABLE_NAME" --region "$REGION"
        echo -e "${GREEN}✅ DynamoDB table is now active${NC}"

        # Enable TTL on the ttl field for automatic session cleanup
        echo -e "${YELLOW}🕒 Enabling TTL for automatic session cleanup...${NC}"
        aws dynamodb update-time-to-live \
            --table-name "$TABLE_NAME" \
            --region "$REGION" \
            --time-to-live-specification Enabled=true,AttributeName=ttl
        echo -e "${GREEN}✅ TTL enabled on 'ttl' attribute${NC}"
    fi

    # Enable TTL for existing tables (if not already enabled)
    echo -e "${YELLOW}🕒 Checking and enabling TTL for session cleanup...${NC}"
    TTL_STATUS=$(aws dynamodb describe-time-to-live \
        --table-name "$TABLE_NAME" \
        --region "$REGION" \
        --query 'TimeToLiveDescription.TimeToLiveStatus' \
        --output text 2>/dev/null || echo "DISABLED")
    
    if [ "$TTL_STATUS" != "ENABLED" ]; then
        aws dynamodb update-time-to-live \
            --table-name "$TABLE_NAME" \
            --region "$REGION" \
            --time-to-live-specification Enabled=true,AttributeName=ttl
        echo -e "${GREEN}✅ TTL enabled on 'ttl' attribute${NC}"
    else
        echo -e "${GREEN}✅ TTL already enabled on 'ttl' attribute${NC}"
    fi

    # Get table ARN
    TABLE_ARN=$(aws dynamodb describe-table \
        --table-name "$TABLE_NAME" \
        --region "$REGION" \
        --query 'Table.TableArn' \
        --output text)

    echo "  Table ARN: $TABLE_ARN"
}

# Function to create IAM role for Lambda
create_lambda_role() {
    echo -e "${YELLOW}📝 Creating Lambda execution role${NC}"

    ROLE_NAME="${STACK_NAME}-lambda-role"

    # Create trust policy
    cat > /tmp/trust-policy.json << EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Service": "lambda.amazonaws.com"
      },
      "Action": "sts:AssumeRole"
    }
  ]
}
EOF

    # Create permissions policy
    cat > /tmp/lambda-permissions.json << EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": "arn:aws:logs:*:*:*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem",
        "dynamodb:Query",
        "dynamodb:Scan"
      ],
      "Resource": "$TABLE_ARN"
    },
    {
      "Effect": "Allow",
      "Action": [
        "sns:Publish",
        "sns:GetTopicAttributes"
      ],
      "Resource": "*",
      "Condition": {
        "StringLike": {
          "sns:TopicName": "mcp-*"
        }
      }
    },
    {
      "Effect": "Allow",
      "Action": [
        "xray:PutTraceSegments",
        "xray:PutTelemetryRecords"
      ],
      "Resource": "*"
    }
  ]
}
EOF

    # Check if role exists
    if aws iam get-role --role-name "$ROLE_NAME" &> /dev/null; then
        echo -e "${YELLOW}⚠️  IAM role $ROLE_NAME already exists${NC}"
    else
        # Create role
        aws iam create-role \
            --role-name "$ROLE_NAME" \
            --assume-role-policy-document file:///tmp/trust-policy.json \
            --tags Key=Project,Value=Lambda-MCP-Server

        echo -e "${GREEN}✅ IAM role created: $ROLE_NAME${NC}"
    fi

    # Update/create policy
    POLICY_NAME="${STACK_NAME}-lambda-policy"

    if aws iam get-role-policy --role-name "$ROLE_NAME" --policy-name "$POLICY_NAME" &> /dev/null; then
        aws iam put-role-policy \
            --role-name "$ROLE_NAME" \
            --policy-name "$POLICY_NAME" \
            --policy-document file:///tmp/lambda-permissions.json
        echo -e "${GREEN}✅ IAM policy updated${NC}"
    else
        aws iam put-role-policy \
            --role-name "$ROLE_NAME" \
            --policy-name "$POLICY_NAME" \
            --policy-document file:///tmp/lambda-permissions.json
        echo -e "${GREEN}✅ IAM policy created${NC}"
    fi

    # Get role ARN
    ROLE_ARN=$(aws iam get-role \
        --role-name "$ROLE_NAME" \
        --query 'Role.Arn' \
        --output text)

    echo "  Role ARN: $ROLE_ARN"

    # Cleanup temp files
    rm -f /tmp/trust-policy.json /tmp/lambda-permissions.json
}

# Function to output environment variables
output_environment_vars() {
    echo -e "${BLUE}📋 Environment Variables for Lambda:${NC}"
    echo ""
    echo "Add these to your Lambda function environment variables:"
    echo ""
    echo -e "${GREEN}SESSION_TABLE_NAME=${NC}$TABLE_NAME"
    if [ ! -z "$TOPIC_ARN" ]; then
        echo -e "${GREEN}SNS_TOPIC_ARN=${NC}$TOPIC_ARN"
    fi
    echo -e "${GREEN}AWS_DEFAULT_REGION=${NC}$REGION"
    echo ""

    # Create .env file for local development
    cat > .env << EOF
# Lambda MCP Server Environment Variables (Clean Architecture)
# 
# For local development with 'cargo lambda watch', you only need:
SESSION_TABLE_NAME=$TABLE_NAME
AWS_DEFAULT_REGION=$REGION

# Optional SNS topic for external global event publishing
# If not set, server uses internal tokio broadcast only
$(if [ ! -z "$TOPIC_ARN" ]; then echo "SNS_TOPIC_ARN=$TOPIC_ARN"; else echo "# SNS_TOPIC_ARN=your-topic-arn-here"; fi)

# AWS Lambda runtime variables (only needed for actual Lambda deployment, not local development)
# AWS_LAMBDA_FUNCTION_NAME=lambda-mcp-server
# AWS_LAMBDA_FUNCTION_MEMORY_SIZE=512
# AWS_LAMBDA_FUNCTION_VERSION=\$LATEST

# Optional configuration overrides
# ALLOWED_ORIGINS=http://localhost:3000,https://your-domain.com
# ENABLE_COMPRESSION=true
# MAX_RESPONSE_SIZE_MB=200
# RUST_LOG=info
EOF

    echo -e "${GREEN}✅ Environment variables saved to .env file${NC}"
    echo ""
    echo -e "${BLUE}🏗️ Architecture Notes:${NC}"
    echo "  • Server uses tokio broadcast channels for internal notifications"
    echo "  • SNS topic is optional for external event publishing"
    echo "  • No SQS queues needed - eliminates fan-out competition issues"
    echo "  • Multiple SSE connections share events via tokio broadcast"
}

# Main execution
echo -e "${BLUE}Starting infrastructure setup...${NC}"
echo ""

create_sns_topic
echo ""

create_dynamodb_table
echo ""

create_lambda_role
echo ""

output_environment_vars
echo ""

echo -e "${GREEN}🎉 Infrastructure setup completed successfully!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "1. Deploy your Lambda function using SAM or cargo-lambda"
echo "2. Set the environment variables shown above"
echo "3. Configure API Gateway to trigger your Lambda"
echo "4. Test your MCP server endpoints with the lambda-mcp-client"
echo ""
echo -e "${BLUE}🧪 Testing the new architecture:${NC}"
echo "  cd ../lambda-mcp-client"
echo "  cargo run -- test --url http://127.0.0.1:9000 --test-sse-streaming"
echo ""
echo -e "${YELLOW}ℹ️  For cleanup, run: ./scripts/cleanup-infrastructure.sh${NC}"
