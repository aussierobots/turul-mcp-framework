# Global Fan-Out Notification Architecture

## Executive Summary

This document describes the architectural design for global fan-out messaging in the MCP Framework, enabling notifications to be distributed across multiple server instances, regions, and deployment environments.

**Current Status**: ✅ **SINGLE-INSTANCE COMPLETE** → 🚧 **MULTI-INSTANCE DESIGN PHASE**

## 🏗️ Three-Tier Notification System

### Tier 1: Local Instance (Current - Working)

```
┌─────────────────────────────────────────────────────────────────────┐
│                      LOCAL INSTANCE TIER                            │
├─────────────────────────────────────────────────────────────────────┤
│ SessionContext → NotificationBroadcaster                            │
│                          ↓                                          │
│ StreamManagerNotificationBroadcaster                                │
│                          ↓                                          │
│ tokio::broadcast channels (per session)                            │
│                          ↓                                          │
│ SSE streams to connected clients ✅                                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Features**: Per-session isolation, resumability, event persistence
**Performance**: ~10K concurrent sessions per instance
**Latency**: <1ms notification delivery

### Tier 2: Multi-Instance (Proposed)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MULTI-INSTANCE TIER                              │
├─────────────────────────────────────────────────────────────────────┤
│ Instance A                   Instance B                   Instance C │
│     ↓                            ↓                            ↓     │
│ CompositeNotificationBroadcaster                                     │
│     ├── Local (Tier 1)                                              │
│     ├── NATS JetStream ──────────┼──────────────────────────────────┤
│     └── Redis Pub/Sub ───────────┼──────────────────────────────────┤
│                                  │                                  │
│ Cross-instance session migration and notification routing           │
└─────────────────────────────────────────────────────────────────────┘
```

**Features**: Horizontal scaling, session migration, cross-instance notifications
**Performance**: ~100K sessions across cluster
**Latency**: <10ms cross-instance delivery

### Tier 3: Global/Cloud (Proposed)

```
┌─────────────────────────────────────────────────────────────────────┐
│                      GLOBAL/CLOUD TIER                              │
├─────────────────────────────────────────────────────────────────────┤
│ Region A Cluster        Region B Cluster        Lambda Functions    │
│        ↓                       ↓                        ↓           │
│ AwsSnsNotificationBroadcaster                                        │
│        ├── SNS Topics for fan-out                                   │
│        ├── SQS FIFO for ordering                                    │
│        ├── EventBridge for routing                                  │
│        └── DynamoDB Streams for state sync                          │
│                                                                     │
│ Global session state synchronization and disaster recovery         │
└─────────────────────────────────────────────────────────────────────┘
```

**Features**: Multi-region, serverless, disaster recovery
**Performance**: Unlimited scale via AWS services
**Latency**: <100ms global delivery

## 🔧 New Notification Broadcaster Implementations

### 1. NatsNotificationBroadcaster

```rust
/// NATS-based notification broadcaster for microservices
pub struct NatsNotificationBroadcaster {
    nats_client: Arc<async_nats::Client>,
    jetstream_context: Arc<async_nats::jetstream::Context>,
    subject_prefix: String,
    cluster_id: String,
}

impl NatsNotificationBroadcaster {
    /// Subject pattern: mcp.notifications.{cluster_id}.{instance_id}.{session_id}.{notification_type}
    fn build_subject(&self, session_id: &str, notification_type: &str) -> String {
        format!("{}.{}.{}.{}", 
            self.subject_prefix, 
            self.cluster_id, 
            session_id, 
            notification_type
        )
    }
    
    /// Subscribe to all notifications for a session across all instances
    async fn subscribe_session_global(&self, session_id: &str) -> Result<Subscription, NatsError> {
        let subject = format!("{}.{}.{}.>", self.subject_prefix, self.cluster_id, session_id);
        self.jetstream_context.subscribe(subject).await
    }
}

#[async_trait]
impl NotificationBroadcaster for NatsNotificationBroadcaster {
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError> {
        let subject = self.build_subject(session_id, &notification.method);
        let payload = serde_json::to_vec(&notification)?;
        
        // Use JetStream for guaranteed delivery
        self.jetstream_context
            .publish(subject, payload.into())
            .await?;
            
        Ok(())
    }
    
    async fn broadcast_to_all_sessions(&self, notification: JsonRpcNotification) -> Result<Vec<String>, BroadcastError> {
        // Wildcard publish to all sessions in cluster
        let subject = format!("{}.{}.*.{}", self.subject_prefix, self.cluster_id, notification.method);
        let payload = serde_json::to_vec(&notification)?;
        
        self.jetstream_context
            .publish(subject, payload.into())
            .await?;
            
        // Return list of notified sessions (requires NATS tracking)
        Ok(vec![]) // TODO: Implement session tracking
    }
}
```

### 2. AwsSnsNotificationBroadcaster

```rust
/// AWS SNS-based notification broadcaster for serverless environments
pub struct AwsSnsNotificationBroadcaster {
    sns_client: Arc<aws_sdk_sns::Client>,
    sqs_client: Arc<aws_sdk_sqs::Client>,
    topic_arn: String,
    fifo_queue_url: Option<String>,
    use_message_deduplication: bool,
}

#[async_trait]
impl NotificationBroadcaster for AwsSnsNotificationBroadcaster {
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError> {
        let message_attributes = HashMap::from([
            ("session_id".to_string(), MessageAttributeValue::builder()
                .data_type("String")
                .string_value(session_id)
                .build()
            ),
            ("notification_type".to_string(), MessageAttributeValue::builder()
                .data_type("String")
                .string_value(&notification.method)
                .build()
            ),
        ]);
        
        let message_body = serde_json::to_string(&notification)?;
        
        if let Some(queue_url) = &self.fifo_queue_url {
            // Use SQS FIFO for ordered delivery
            self.sqs_client
                .send_message()
                .queue_url(queue_url)
                .message_body(&message_body)
                .message_group_id(session_id) // FIFO ordering per session
                .message_deduplication_id(format!("{}_{}", session_id, notification.id))
                .send()
                .await?;
        } else {
            // Use SNS for fan-out
            self.sns_client
                .publish()
                .topic_arn(&self.topic_arn)
                .message(&message_body)
                .set_message_attributes(Some(message_attributes))
                .send()
                .await?;
        }
        
        Ok(())
    }
}
```

### 3. CompositeNotificationBroadcaster

```rust
/// Composite broadcaster that routes to multiple backends based on configuration
pub struct CompositeNotificationBroadcaster {
    broadcasters: Vec<Arc<dyn NotificationBroadcaster>>,
    routing_rules: Vec<RoutingRule>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
}

#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Condition to match (session pattern, notification type, etc.)
    pub condition: RoutingCondition,
    /// Target broadcasters to route to
    pub targets: Vec<String>,
    /// Whether to continue to next rule if this one matches
    pub continue_on_match: bool,
}

#[derive(Debug, Clone)]
pub enum RoutingCondition {
    SessionPattern(String),     // e.g., "lambda-*", "web-*"
    NotificationType(String),   // e.g., "notifications/progress"
    SessionCount(usize),        // Route based on session count
    Always,                     // Always match
}

impl CompositeNotificationBroadcaster {
    pub fn new() -> Self {
        Self {
            broadcasters: Vec::new(),
            routing_rules: Vec::new(),
            circuit_breakers: HashMap::new(),
        }
    }
    
    pub fn add_broadcaster(mut self, name: String, broadcaster: Arc<dyn NotificationBroadcaster>) -> Self {
        self.broadcasters.push(broadcaster);
        self.circuit_breakers.insert(name, CircuitBreaker::new());
        self
    }
    
    pub fn add_routing_rule(mut self, rule: RoutingRule) -> Self {
        self.routing_rules.push(rule);
        self
    }
}

#[async_trait]
impl NotificationBroadcaster for CompositeNotificationBroadcaster {
    async fn send_notification(
        &self,
        session_id: &str,
        notification: JsonRpcNotification,
    ) -> Result<(), BroadcastError> {
        let mut results = Vec::new();
        
        for rule in &self.routing_rules {
            if rule.matches(session_id, &notification) {
                for target_name in &rule.targets {
                    if let Some(broadcaster) = self.get_broadcaster(target_name) {
                        if self.circuit_breakers[target_name].is_closed() {
                            match broadcaster.send_notification(session_id, notification.clone()).await {
                                Ok(_) => {
                                    self.circuit_breakers[target_name].record_success();
                                    results.push(Ok(()));
                                }
                                Err(e) => {
                                    self.circuit_breakers[target_name].record_failure();
                                    results.push(Err(e));
                                }
                            }
                        } else {
                            results.push(Err(BroadcastError::CircuitBreakerOpen(target_name.clone())));
                        }
                    }
                }
                
                if !rule.continue_on_match {
                    break;
                }
            }
        }
        
        // Return success if at least one broadcaster succeeded
        if results.iter().any(|r| r.is_ok()) {
            Ok(())
        } else {
            Err(BroadcastError::AllBroadcastersFailed(results))
        }
    }
}
```

## 🛠️ Configuration Schema

### Framework Configuration

```toml
[notification]
# Strategy: local | nats | aws | composite
strategy = "composite"
# Fallback behavior when primary fails
fallback_strategy = "local"
# Global timeout for notification delivery
timeout_ms = 5000

[notification.local]
enabled = true
priority = 1
max_sessions_per_instance = 10000

[notification.nats]
enabled = true
priority = 2
servers = ["nats://nats-1:4222", "nats://nats-2:4222"]
cluster_id = "mcp-cluster"
client_id = "mcp-instance-{instance_id}"
subject_prefix = "mcp.notifications"
use_jetstream = true
stream_name = "mcp_notifications"
max_age = "24h"
max_msgs = 1000000

[notification.aws]
enabled = false
priority = 3
region = "us-east-1"
sns_topic_arn = "arn:aws:sns:us-east-1:123456789012:mcp-notifications"
sqs_queue_url = "https://sqs.us-east-1.amazonaws.com/123456789012/mcp-sessions.fifo"
use_fifo = true
deduplication_window_seconds = 300
visibility_timeout_seconds = 30

[notification.routing]
# Route progress notifications to NATS for real-time updates
[[notification.routing.rules]]
condition = { type = "notifications/progress" }
targets = ["local", "nats"]
continue_on_match = false

# Route Lambda session notifications to AWS
[[notification.routing.rules]]
condition = { session_pattern = "lambda-*" }
targets = ["aws"]
continue_on_match = true

# Default: everything to local
[[notification.routing.rules]]
condition = "always"
targets = ["local"]
continue_on_match = false
```

### Infrastructure as Code

```yaml
# docker-compose.yml for NATS cluster
version: '3.8'
services:
  nats-1:
    image: nats:alpine
    command: --jetstream --cluster_name=mcp --cluster=nats://0.0.0.0:6222 --routes=nats-route://nats-2:6222
    ports: ["4222:4222", "6222:6222"]
    
  nats-2:
    image: nats:alpine
    command: --jetstream --cluster_name=mcp --cluster=nats://0.0.0.0:6222 --routes=nats-route://nats-1:6222
    ports: ["4223:4222", "6223:6222"]
```

```terraform
# AWS infrastructure for global fan-out
resource "aws_sns_topic" "mcp_notifications" {
  name = "mcp-notifications"
  
  message_retention_seconds = 1209600  # 14 days
}

resource "aws_sqs_queue" "mcp_sessions" {
  name = "mcp-sessions.fifo"
  fifo_queue = true
  content_based_deduplication = true
  
  message_retention_seconds = 1209600
  visibility_timeout_seconds = 30
}

resource "aws_eventbridge_rule" "mcp_notifications" {
  name = "mcp-notification-routing"
  event_pattern = jsonencode({
    source = ["mcp.framework"]
    detail-type = ["Session Notification"]
  })
}
```

## 🔄 Migration Strategy

### Phase 1: Foundation (Week 1-2)
1. Create `turul-mcp-nats-bridge` crate
2. Implement `NatsNotificationBroadcaster` with JetStream
3. Add NATS integration tests with embedded server
4. Update documentation with NATS examples

### Phase 2: AWS Integration (Week 3-4)  
1. Create `turul-mcp-aws-bridge` crate
2. Extract SNS processor from Lambda example to framework
3. Implement `AwsSnsNotificationBroadcaster` with SQS FIFO
4. Add DynamoDB Streams integration for session state sync

### Phase 3: Composite Routing (Week 5-6)
1. Implement `CompositeNotificationBroadcaster`
2. Add circuit breaker pattern with exponential backoff
3. Create configuration-driven routing engine
4. Implement metrics and observability hooks

### Phase 4: Production Hardening (Week 7-8)
1. Add comprehensive error handling and retries
2. Implement graceful degradation strategies
3. Create monitoring dashboards and alerts
4. Performance testing with 100K+ sessions

## 📊 Architecture Decision Records

### ADR-006: Global Fan-Out Architecture

**Status**: APPROVED  
**Date**: 2025-08-30  
**Context**: MCP Framework needs to scale beyond single instances while maintaining session isolation and notification guarantees.

**Decision**: Implement three-tier notification architecture with pluggable broadcasters.

**Consequences**:
- ✅ Horizontal scaling without architectural changes
- ✅ Gradual migration path from single to multi-instance
- ✅ Cloud-native serverless compatibility
- ⚠️ Increased operational complexity
- ⚠️ New failure modes requiring circuit breakers

### ADR-007: NATS JetStream for Multi-Instance

**Status**: APPROVED  
**Date**: 2025-08-30  
**Context**: Need reliable message delivery between MCP instances with ordering guarantees.

**Decision**: Use NATS JetStream as primary multi-instance transport.

**Rationale**:
- Guaranteed delivery with acknowledgment
- Natural clustering and high availability  
- Subject-based routing matches MCP session model
- Excellent Rust ecosystem support

**Alternatives Considered**:
- Apache Kafka: Too heavy for MCP notification patterns
- Redis Pub/Sub: No delivery guarantees
- RabbitMQ: More complex operational model

### ADR-008: Circuit Breaker Pattern

**Status**: APPROVED  
**Date**: 2025-08-30  
**Context**: Multi-broadcaster setup needs resilience against partial failures.

**Decision**: Implement circuit breaker pattern with automatic fallback to local delivery.

**Implementation**: 
- Open circuit after 5 consecutive failures
- Half-open state after 30-second cooldown  
- Success rate monitoring for dynamic thresholds

## 🧪 Testing Strategy

### Unit Tests
- Each broadcaster implementation tested independently
- Mock dependencies for AWS SDK and NATS client
- Error condition simulation and recovery testing

### Integration Tests  
- Embedded NATS server for NATS broadcaster testing
- LocalStack for AWS service testing
- Multi-broadcaster scenario verification

### Load Tests
- 1000 concurrent sessions with 100 notifications/second
- Cross-instance session migration under load
- Circuit breaker behavior under partial failures

### Chaos Engineering
- Network partition simulation between instances
- Random service failures and recovery verification
- Message delivery guarantee validation

## 📈 Performance Characteristics

| Metric | Local | NATS | AWS SNS/SQS | Target |
|--------|-------|------|-------------|---------|
| **Latency (p99)** | <1ms | <10ms | <100ms | <1s |
| **Throughput** | 10K/sec | 50K/sec | 100K/sec | 1M/sec |
| **Sessions/Instance** | 10K | N/A | N/A | 100K |
| **Cross-Instance Delivery** | N/A | <50ms | <200ms | <1s |
| **Reliability** | 99.9% | 99.99% | 99.999% | >99.9% |

## 🔍 Monitoring and Observability

### Metrics
- Notification delivery latency (per broadcaster)
- Circuit breaker state changes
- Session distribution across instances
- Message queue depths and processing rates

### Logging
- Structured logs with correlation IDs
- Notification routing decisions
- Circuit breaker state transitions
- Session migration events

### Distributed Tracing
- End-to-end notification flow tracing
- Cross-service correlation in AWS environment
- Performance bottleneck identification

## 🚀 Future Enhancements

### Session Affinity
- Implement session-to-instance affinity for reduced cross-instance traffic
- Load balancer configuration for sticky sessions
- Graceful session migration during instance maintenance

### Notification Filtering  
- Client-side notification filtering based on subscription patterns
- Server-side filtering to reduce bandwidth
- Dynamic subscription management

### Edge Computing
- CDN-based notification delivery for global clients
- Regional notification caches
- Intelligent routing based on client geography

This architecture provides a solid foundation for scaling MCP Framework notifications to enterprise and cloud-native environments while maintaining the simplicity and reliability of the current single-instance implementation.