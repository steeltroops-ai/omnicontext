---
title: Enterprise
description: Enterprise deployment, security, and support for organizations
category: Enterprise
order: 50
---

# Enterprise

OmniContext Enterprise extends the free, open-source core engine with managed cloud infrastructure, team collaboration, and compliance features for organizations that operate at scale.

> **The OSS core is always free.** Enterprise plans add hosting, centralized indexes, and SLA-backed support on top of the same battle-tested Rust engine.

---

## Why Enterprise?

| Scenario | Free Core | Enterprise |
|----------|-----------|------------|
| Single developer, local machine | ✅ Perfect | Not needed |
| Team of 5–20 sharing an index | ✅ Self-host daemon | ✅ Managed cloud |
| 50+ developers, multiple repos | Manual setup | ✅ Managed + RBAC |
| SOC 2 / GDPR audit requirements | Manual docs | ✅ Certified |
| On-premise air-gapped deployment | ✅ Always possible | ✅ Helm + Docker |

---

## Enterprise Features

### Managed Multi-Repo Index

Deploy OmniContext as a centralized service your entire team queries via REST or MCP.

**Benefits**:
- No local installation required for team members
- Centralized, always-fresh indexes
- Consistent performance regardless of developer machine specs
- Automatic incremental re-indexing on push via GitHub / GitLab webhooks

**Deployment options**:
- Kubernetes (official Helm chart)
- Docker / Docker Compose
- AWS ECS · GCP Cloud Run · Azure Container Instances
- Air-gapped on-premise (bare metal or VMware)

### Cross-Repository Intelligence

OmniContext was designed from the ground up for monorepos and multi-service architectures.

- Unified hybrid search across all repositories simultaneously
- Cross-repo dependency analysis and blast-radius computation
- Shared symbol resolution (e.g., find every service that calls `UserService.authenticate`)
- Monorepo support with per-package granularity

### Team Collaboration

- Centralized index storage with real-time incremental sync
- Shared search history and bookmarked queries
- Per-repository access permissions
- Collaborative architectural annotations

### Security & Compliance

**Access Control**:
- Role-based access control (RBAC) — Owner / Admin / Developer / Viewer
- Repository-level and document-level security filtering
- API key lifecycle management with scopes and expiry
- Audit log for every search query and config change

**Authentication**:
- SAML 2.0 (Okta, Azure AD, Google Workspace, Ping)
- OAuth 2.0 / OpenID Connect
- LDAP / Active Directory
- Multi-factor authentication (TOTP, WebAuthn)
- IP allowlists

**Compliance**:
- SOC 2 Type II certified (report available under NDA)
- GDPR compliant — EU data residency available
- HIPAA-ready deployment option
- Comprehensive audit logs with retention policies

### Performance & Reliability

**SLA Commitments** (Growth and above):
- 99.9% uptime guarantee (99.99% on Enterprise)
- <100ms P99 search latency for indexes up to 50M chunks
- 24/7 technical support with escalation path
- Dedicated customer success manager

**Scalability**:
- Horizontal scaling — add index workers without downtime
- Read replicas for high-traffic teams
- Auto-scaling based on query volume
- Multi-region active-active deployment

---

## Deployment

### Kubernetes (Helm)

```bash
# Add the OmniContext Helm repository
helm repo add omnicontext https://charts.omnicontext.dev
helm repo update

# Production install with 3 replicas
helm install omnicontext omnicontext/omnicontext \
  --set replicas=3 \
  --set resources.limits.memory=8Gi \
  --set ingress.enabled=true \
  --set auth.saml.enabled=true \
  --set storage.class=standard
```

### Docker / Docker Compose

```bash
# Pull latest image
docker pull omnicontext/server:latest

# Run with persistent storage and auth enabled
docker run -d \
  -p 8080:8080 \
  -v /data/omnicontext:/data \
  -e OMNI_AUTH_ENABLED=true \
  -e OMNI_SAML_METADATA_URL=https://your-idp/metadata \
  omnicontext/server:latest
```

### Cloud-Native Quickstart

**AWS** (ECS + Fargate):
```bash
# Deploy via CloudFormation template
aws cloudformation create-stack \
  --stack-name omnicontext-enterprise \
  --template-url https://omnicontext.dev/cfn/latest.yaml \
  --parameters ParameterKey=Environment,ParameterValue=production
```

**GCP** (Cloud Run):
```bash
gcloud run deploy omnicontext \
  --image gcr.io/omnicontext/server:latest \
  --region us-central1 \
  --memory 4Gi \
  --concurrency 80
```

---

## Pricing

### Cloud Starter — $29 / developer / month
- Up to 10 repositories
- 50 GB index storage
- 500K API calls / month
- Email support (48-hour response)
- Community Slack access

### Cloud Team — $59 / developer / month
- Unlimited repositories
- 500 GB index storage included
- 5M API calls / month included
- Cross-repo dependency analysis
- SAML / OAuth SSO
- Priority support (4-hour SLA)
- Dedicated team Slack channel

### Enterprise Plan — Custom Pricing
- 100+ developers
- Unlimited storage and API calls
- On-premise / air-gapped deployment option
- Custom SLA (up to 99.99% uptime)
- Dedicated account manager + CSM
- Quarterly business reviews
- Custom integration development
- Volume discounts

Contact [enterprise@omnicontext.dev](mailto:enterprise@omnicontext.dev) for a tailored quote.

---

## Support Tiers

| Tier | Response Time | Channels | Included In |
|------|--------------|----------|-------------|
| Community | Best effort | GitHub Issues | Free Core |
| Standard | 48 hours | Email | Cloud Starter |
| Priority | 4 hours | Email + Slack | Cloud Team |
| Premium | 1 hour | Email + Slack + Phone | Enterprise |
| Platinum | 15 minutes | Dedicated hotline | Enterprise (custom) |

### Professional Services

- Custom MCP tool development
- Migration from Sourcegraph / Codeium / Copilot
- Team training and onboarding workshops
- Architecture consulting and performance tuning
- CI/CD pipeline integration

---

## Getting Started

### Request a Demo

Schedule a 30-minute personalized walkthrough:
- Email: [enterprise@omnicontext.dev](mailto:enterprise@omnicontext.dev)
- Calendar: [Book a demo](https://omnicontext.dev/demo)

### 30-Day Free Trial

Start a full Enterprise trial — no credit card required:
- All Enterprise features enabled
- Up to 25 developers
- Full migration assistance included
- Dedicated onboarding engineer

### Migration Support

Switching from another tool? We provide:
- Automated data migration scripts
- Side-by-side performance benchmarking
- Custom integration development for your existing CI/CD
- Dedicated migration engineer for the first 90 days
