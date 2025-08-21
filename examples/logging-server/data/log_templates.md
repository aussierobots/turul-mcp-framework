# Application Logging Templates and Best Practices

## Overview

This document provides comprehensive logging templates and best practices for development teams. These templates ensure consistent, searchable, and compliant logging across all application components.

## Log Message Templates

### Authentication Events

#### Successful Login
```json
{
  "timestamp": "2025-01-19T10:30:00.123Z",
  "level": "INFO",
  "category": "security",
  "event_type": "authentication.login_success",
  "user_id": "user_12345",
  "username": "john.doe@company.com",
  "session_id": "sess_abcd1234",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
  "location": {
    "country": "US",
    "city": "New York",
    "coordinates": [40.7128, -74.0060]
  },
  "auth_method": "password",
  "mfa_used": true,
  "risk_score": 0.1,
  "message": "User successfully authenticated",
  "correlation_id": "req_xyz789"
}
```

#### Failed Login
```json
{
  "timestamp": "2025-01-19T10:31:30.456Z",
  "level": "WARN",
  "category": "security", 
  "event_type": "authentication.login_failure",
  "username": "john.doe@company.com",
  "ip_address": "192.168.1.100",
  "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
  "failure_reason": "invalid_password",
  "attempt_number": 3,
  "account_locked": false,
  "risk_score": 0.7,
  "message": "Authentication failed - invalid password",
  "correlation_id": "req_abc123"
}
```

### Business Logic Events

#### User Registration
```json
{
  "timestamp": "2025-01-19T10:25:15.789Z",
  "level": "INFO",
  "category": "business",
  "event_type": "user_management.user_creation",
  "user_id": "user_67890",
  "username": "jane.smith@company.com",
  "role": "customer",
  "created_by": "system",
  "registration_method": "web_form",
  "email_verified": false,
  "terms_accepted": true,
  "marketing_consent": false,
  "ip_address": "203.0.113.42",
  "message": "New user account created successfully",
  "correlation_id": "reg_def456"
}
```

#### Payment Processing
```json
{
  "timestamp": "2025-01-19T14:22:45.321Z",
  "level": "INFO",
  "category": "business",
  "event_type": "financial_transactions.payment_processing",
  "transaction_id": "txn_987654321",
  "user_id": "user_12345",
  "amount": 99.99,
  "currency": "USD",
  "payment_method": "credit_card_****1234",
  "processor": "stripe",
  "processor_response": "approved",
  "processing_time_ms": 1247,
  "fraud_score": 0.15,
  "message": "Payment processed successfully",
  "correlation_id": "pay_ghi789"
}
```

### System Events

#### Database Query Performance
```json
{
  "timestamp": "2025-01-19T11:15:30.654Z",
  "level": "DEBUG",
  "category": "performance",
  "event_type": "database.query_executed",
  "query_type": "SELECT",
  "table": "users",
  "execution_time_ms": 45,
  "rows_affected": 1,
  "index_used": true,
  "cache_hit": false,
  "connection_pool_size": 20,
  "query_hash": "sha256:abc123def456",
  "message": "Database query executed",
  "correlation_id": "db_jkl012"
}
```

#### API Request/Response
```json
{
  "timestamp": "2025-01-19T13:45:12.987Z",
  "level": "INFO",
  "category": "api",
  "event_type": "api.request_processed",
  "method": "POST",
  "endpoint": "/api/v1/users",
  "status_code": 201,
  "response_time_ms": 156,
  "request_size_bytes": 512,
  "response_size_bytes": 248,
  "user_id": "user_12345",
  "ip_address": "192.168.1.100",
  "user_agent": "MyApp/1.0",
  "rate_limit_remaining": 98,
  "message": "API request processed successfully",
  "correlation_id": "api_mno345"
}
```

### Security Events

#### Suspicious Activity Detection
```json
{
  "timestamp": "2025-01-19T16:30:45.123Z",
  "level": "ERROR",
  "category": "security",
  "event_type": "security_events.suspicious_activity",
  "activity_type": "sql_injection_attempt",
  "user_id": null,
  "ip_address": "198.51.100.10",
  "user_agent": "curl/7.68.0",
  "endpoint": "/api/v1/users",
  "attempted_payload": "'; DROP TABLE users; --",
  "blocked": true,
  "threat_level": "high",
  "detection_method": "waf_rule",
  "rule_id": "OWASP_942100",
  "message": "SQL injection attempt detected and blocked",
  "correlation_id": "sec_pqr678"
}
```

### Error Events

#### Application Exception
```json
{
  "timestamp": "2025-01-19T09:45:22.456Z",
  "level": "ERROR",
  "category": "error",
  "event_type": "application.exception",
  "exception_type": "NullPointerException",
  "exception_message": "User object is null",
  "stack_trace": "java.lang.NullPointerException: User object is null\n    at UserService.processUser(UserService.java:45)",
  "service": "user_service",
  "method": "processUser",
  "user_id": "user_12345",
  "request_id": "req_stu901",
  "session_id": "sess_vwx234",
  "message": "Unhandled exception occurred during user processing",
  "correlation_id": "err_yz567"
}
```

## Log Level Guidelines

### DEBUG
- Development and troubleshooting information
- Variable values and execution flow
- Performance profiling data
- Only enabled in development environments

**Example Use Cases:**
- Function entry/exit points
- Variable state changes
- Database query details
- Cache hit/miss ratios

### INFO  
- Normal application flow
- Business events and milestones
- User actions and system events
- Configuration changes

**Example Use Cases:**
- User login/logout
- Successful transactions
- Service startup/shutdown
- Configuration loading

### WARN
- Unexpected situations that don't stop execution
- Deprecated feature usage
- Recoverable errors
- Performance issues

**Example Use Cases:**
- Failed retry attempts
- Rate limiting triggered
- Missing optional configuration
- Slow query warnings

### ERROR
- Application errors and exceptions
- Failed operations
- Integration failures
- Data validation errors

**Example Use Cases:**
- Unhandled exceptions
- Database connection failures
- External API failures
- Payment processing errors

### CRITICAL
- System failures and security breaches
- Data corruption
- Service unavailability
- Compliance violations

**Example Use Cases:**
- Security incidents
- Data breaches
- System crashes
- Audit failures

## Best Practices

### Message Structure
1. **Consistency**: Use consistent field names across all logs
2. **Context**: Include correlation IDs for request tracing
3. **Searchability**: Use structured JSON format
4. **Completeness**: Include all relevant context information
5. **Timeliness**: Log events as they occur

### Security Considerations
1. **No Secrets**: Never log passwords, API keys, or tokens
2. **PII Protection**: Mask or exclude personally identifiable information
3. **Data Classification**: Mark sensitive data appropriately
4. **Access Control**: Restrict log access to authorized personnel
5. **Encryption**: Encrypt logs in transit and at rest

### Performance Guidelines
1. **Async Logging**: Use asynchronous logging to avoid blocking
2. **Sampling**: Sample high-frequency debug logs in production
3. **Buffering**: Buffer logs for batch processing
4. **Compression**: Compress archived logs
5. **Retention**: Implement appropriate retention policies

### Compliance Requirements

#### GDPR Compliance
- Log data processing activities
- Include lawful basis for processing
- Support data subject access requests
- Implement data retention policies
- Enable data deletion requests

#### SOX Compliance
- Maintain immutable audit trails
- Log all financial data changes
- Include approval workflows
- Preserve logs for 7 years
- Implement segregation of duties

#### PCI DSS Compliance
- Log access to cardholder data
- Monitor and test security systems
- Implement secure log storage
- Regular log reviews
- Incident response procedures

## Log Analysis Patterns

### Common Queries

#### Security Analysis
```sql
-- Failed login attempts by IP
SELECT ip_address, COUNT(*) as failed_attempts
FROM logs 
WHERE event_type = 'authentication.login_failure'
  AND timestamp > NOW() - INTERVAL 1 HOUR
GROUP BY ip_address
HAVING COUNT(*) > 5
ORDER BY failed_attempts DESC;

-- Suspicious activity patterns
SELECT activity_type, COUNT(*) as incidents
FROM logs
WHERE category = 'security'
  AND level = 'ERROR'
  AND timestamp > NOW() - INTERVAL 24 HOUR
GROUP BY activity_type;
```

#### Performance Analysis
```sql
-- Slow API endpoints
SELECT endpoint, AVG(response_time_ms) as avg_response_time
FROM logs
WHERE event_type = 'api.request_processed'
  AND timestamp > NOW() - INTERVAL 1 HOUR
GROUP BY endpoint
HAVING AVG(response_time_ms) > 1000
ORDER BY avg_response_time DESC;

-- Database performance issues
SELECT table_name, AVG(execution_time_ms) as avg_exec_time
FROM logs
WHERE event_type = 'database.query_executed'
  AND timestamp > NOW() - INTERVAL 1 HOUR
GROUP BY table_name
ORDER BY avg_exec_time DESC;
```

#### Business Intelligence
```sql
-- User registration trends
SELECT DATE(timestamp) as registration_date, COUNT(*) as new_users
FROM logs
WHERE event_type = 'user_management.user_creation'
  AND timestamp > NOW() - INTERVAL 30 DAY
GROUP BY DATE(timestamp)
ORDER BY registration_date;

-- Payment success rates
SELECT 
  DATE(timestamp) as payment_date,
  COUNT(*) as total_payments,
  SUM(CASE WHEN processor_response = 'approved' THEN 1 ELSE 0 END) as successful_payments,
  ROUND(100.0 * SUM(CASE WHEN processor_response = 'approved' THEN 1 ELSE 0 END) / COUNT(*), 2) as success_rate
FROM logs
WHERE event_type = 'financial_transactions.payment_processing'
  AND timestamp > NOW() - INTERVAL 7 DAY
GROUP BY DATE(timestamp)
ORDER BY payment_date;
```

## Alert Rules and Thresholds

### Critical Alerts (Immediate Response)
- Security incidents: Any critical security event
- System failures: Service unavailability > 1 minute
- Data breaches: Unauthorized data access
- Payment failures: Payment success rate < 95%

### Warning Alerts (Response within 15 minutes)
- High error rate: > 5% in 5-minute window
- Performance degradation: Response time > 2x baseline
- Failed authentications: > 10 failures from single IP
- Database issues: Connection failures > 3 in 1 minute

### Info Alerts (Response within 1 hour)
- Deployment events: New version deployments
- Configuration changes: System configuration updates
- Capacity warnings: Resource usage > 80%
- Business milestones: Revenue targets reached

This comprehensive logging framework ensures consistent, compliant, and actionable logging across all application components while supporting security, compliance, and operational requirements.