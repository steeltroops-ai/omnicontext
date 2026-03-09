---
title: Enterprise
description: Enterprise deployment, security, and support for organizations
category: Enterprise
order: 50
---

# Enterprise

OmniContext Enterprise provides hosted deployment, team collaboration, and enterprise-grade security for organizations.

## Enterprise Features

### Hosted REST API

Deploy OmniContext as a centralized service accessible via REST API.

**Benefits**:
- No local installation required
- Centralized index management
- Consistent performance across team
- Automatic updates and maintenance

**Deployment Options**:
- Kubernetes with Helm charts
- Docker containers
- Cloud-native (AWS, GCP, Azure)

### Multi-Repository Workspaces

Index multiple repositories with cross-repo search and dependency tracking.

**Capabilities**:
- Unified search across all repositories
- Cross-repo dependency analysis
- Shared symbol resolution
- Monorepo support

### Team Collaboration

Share indexes across your organization for instant access.

**Features**:
- Centralized index storage
- Real-time synchronization
- Team-wide search history
- Collaborative annotations

### Security & Compliance

Enterprise-grade security and compliance features.

**Access Control**:
- Role-based access control (RBAC)
- Document-level security filtering
- Repository-level permissions
- API key management

**Authentication**:
- SAML 2.0 integration
- OAuth 2.0 / OpenID Connect
- LDAP / Active Directory
- Multi-factor authentication (MFA)

**Audit & Compliance**:
- Comprehensive audit logs
- Query attribution and tracking
- Data retention policies
- SOC 2 Type II certified
- GDPR compliant

### Performance & Reliability

Production-grade performance with SLA guarantees.

**SLA Commitments**:
- 99.9% uptime guarantee
- <100ms P99 search latency
- 24/7 technical support
- Dedicated account manager

**Scalability**:
- Horizontal scaling
- Load balancing
- Auto-scaling based on demand
- Multi-region deployment

## Deployment

### Kubernetes

Deploy with Helm charts for production environments:

```bash
# Add Helm repository
helm repo add omnicontext https://charts.omnicontext.dev
helm repo update

# Install with custom values
helm install omnicontext omnicontext/omnicontext \
  --set replicas=3 \
  --set resources.limits.memory=8Gi \
  --set ingress.enabled=true
```

### Docker

Run as containerized service:

```bash
# Pull latest image
docker pull omnicontext/server:latest

# Run with persistent storage
docker run -d \
  -p 8080:8080 \
  -v /data/omnicontext:/data \
  -e OMNI_AUTH_ENABLED=true \
  omnicontext/server:latest
```

### Cloud Providers

Pre-configured deployments for major cloud platforms:

**AWS**:
- ECS/Fargate deployment
- RDS for metadata storage
- S3 for vector storage
- CloudWatch integration

**GCP**:
- Cloud Run deployment
- Cloud SQL for metadata
- Cloud Storage for vectors
- Cloud Monitoring integration

**Azure**:
- Container Instances
- Azure SQL Database
- Blob Storage for vectors
- Application Insights integration

## Pricing

Enterprise pricing is usage-based with volume discounts:

**Base Plan**: $500/month
- Up to 10 developers
- 100GB storage included
- 1M API calls/month included
- Email support

**Growth Plan**: $50/developer/month
- 11+ developers
- Additional storage: $0.10/GB/month
- Additional API calls: $0.001 per 1000
- Priority support

**Enterprise Plan**: Custom pricing
- 100+ developers
- Volume discounts
- Custom SLA
- Dedicated support
- On-premise deployment option

## Support

### Support Tiers

**Standard** (included):
- Email support
- 24-hour response time
- Community forums

**Priority** (Growth+):
- Email and chat support
- 4-hour response time
- Dedicated Slack channel

**Premium** (Enterprise):
- 24/7 phone support
- 1-hour response time
- Dedicated account manager
- Quarterly business reviews

### Professional Services

- Custom integration development
- Training and onboarding
- Performance optimization
- Architecture consulting

## Getting Started

### Request Demo

Schedule a personalized demo with our team:
- Email: [enterprise@omnicontext.dev](mailto:enterprise@omnicontext.dev)
- Calendar: [Book a demo](https://omnicontext.dev/demo)

### Trial

Start a 30-day free trial:
- Full enterprise features
- Up to 10 developers
- No credit card required
- Migration assistance included

### Migration

We provide full migration support:
- Data migration from existing tools
- Custom integration development
- Team training and onboarding
- Dedicated migration engineer
