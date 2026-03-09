---
title: Enterprise
description: Deployment and features for organizations
category: Enterprise
order: 30
---

# Enterprise

OmniContext Enterprise provides hosted deployment, team collaboration, and enterprise security.

## Features

### Hosted REST API
Deploy OmniContext as a centralized service. Your team connects via REST API instead of running local instances.

### Multi-repository workspaces
Index multiple repositories with cross-repo search and dependency tracking.

### Team collaboration
Share indexes across your organization. Developers get instant access to indexed codebases.

### Security & compliance
- **RBAC**: Role-based access control
- **Document-level security**: Filter search results by user permissions
- **Audit logs**: Track all queries with user attribution
- **SSO**: SAML and OAuth integration

### SLA guarantees
- 99.9% uptime
- < 100ms P99 search latency
- 24/7 support
- Dedicated account manager

## Deployment

### Kubernetes
Deploy with Helm charts. Horizontal scaling via load balancer.

```bash
helm repo add omnicontext https://charts.omnicontext.dev
helm install omnicontext omnicontext/omnicontext
```

### Docker
Run as containerized service.

```bash
docker run -p 8080:8080 omnicontext/server:latest
```

### Cloud providers
Pre-configured deployments for AWS, GCP, and Azure.

## Pricing

Enterprise pricing is usage-based:

- **Base**: $500/month (up to 10 developers)
- **Per developer**: $50/month (11+ developers)
- **Storage**: $0.10/GB/month
- **API calls**: $0.001 per 1000 calls

Volume discounts available for 100+ developers.

## Contact sales

Schedule a demo: [enterprise@omnicontext.dev](mailto:enterprise@omnicontext.dev)
