---
title: Pricing
category: Enterprise
order: 51
---

# Pricing

OmniContext is built on a **Free Core / Paid Cloud** model. The full engine — every feature, every language, every MCP tool — runs locally at zero cost. Cloud services and enterprise support are optional add-ons for teams that need them.

## Free Core (Open Source, Forever)

The complete OmniContext engine, open source under the Apache 2.0 License. No trial, no seat limits, no telemetry.

| Feature | Included |
|---------|----------|
| Unlimited repositories | ✅ |
| Unlimited file indexing | ✅ |
| All 16 MCP tools | ✅ |
| Hybrid semantic + keyword search | ✅ |
| Local ONNX embedding models | ✅ |
| Real-time file watching | ✅ |
| Dependency graph & blast radius | ✅ |
| Git history intelligence | ✅ |
| VS Code extension | ✅ |
| Universal IDE auto-configuration | ✅ |
| Community support via GitHub | ✅ |

**Cost**: $0 / month — forever.

**Install in one line:**

```bash
# Linux / macOS
curl -fsSL https://omnicontext.dev/install.sh | sh

# Windows (PowerShell)
irm https://omnicontext.dev/install.ps1 | iex

# Cargo
cargo install omnicontext
```

**Source Code**: [github.com/steeltroops-ai/omnicontext](https://github.com/steeltroops-ai/omnicontext)

---

## Paid Cloud (Coming Soon)

For teams that want zero-infrastructure search across their entire org's codebase:

### Cloud Starter — $29 / developer / month
- Hosted multi-repo index (up to 10 repos)
- Shared team search history
- REST API access
- Email support

### Cloud Team — $59 / developer / month
- Unlimited repos
- Cross-repo dependency analysis
- Priority indexing queue
- 4-hour support SLA
- SSO (SAML 2.0 / OAuth 2.0)

### Cloud Enterprise — Custom Pricing
- On-premise or private cloud deployment
- RBAC, LDAP / Active Directory
- SOC 2 Type II, GDPR compliance
- Dedicated account manager
- Custom SLA (up to 99.99% uptime)

**The OSS core always remains free.** Cloud plans add shared hosting, team collaboration, and enterprise compliance on top of the same Rust engine.

Contact: [enterprise@omnicontext.dev](mailto:enterprise@omnicontext.dev)

---

## Comparison with Alternatives

| Feature | OmniContext Core | OmniContext Cloud | Sourcegraph | GitHub Copilot |
|---------|-----------------|-------------------|-------------|----------------|
| **Cost** | **Free** | From $29/dev/mo | $99+/dev/mo | $10–19/dev/mo |
| **Deployment** | 100% Local | Hosted / On-prem | Cloud/Self-hosted | Cloud |
| **Privacy** | 100% Local — no telemetry | Your VPC / private | Data sent to cloud | Data sent to cloud |
| **Search Latency** | <50ms | <100ms | 200–500ms | 300–1000ms |
| **Open Source** | ✅ Apache 2.0 | Engine is OSS | ❌ Proprietary | ❌ Proprietary |
| **MCP Native** | ✅ 16 tools | ✅ 16 tools | ❌ No | ❌ No |
| **Offline** | ✅ Always | ❌ Requires network | ❌ | ❌ |

---

## Frequently Asked Questions

### Is OmniContext really free?

Yes. The core engine is Apache 2.0 open source. You can use it for personal or commercial purposes, self-host it, modify it, and ship it in your own products — all at zero cost.

### What does "Free Core / Paid Cloud" mean?

The Rust engine that runs on your machine (indexing, search, MCP server, daemon, FFI, VS Code extension) will **always** be free and open source. Paid Cloud plans add managed infrastructure — centralized multi-repo indexes, REST APIs, team dashboards — on top of that same engine.

### How do you make money?

Through optional Cloud and Enterprise plans. The OSS engine is our best marketing — it's also genuinely the best local code intelligence tool available.

### Can I use OmniContext commercially?

Yes. Apache 2.0 allows commercial use without restriction. You can even embed it in a commercial product.

### What about the embedding models?

All models (Jina v2 base code, ~550 MB) are downloaded once and run entirely on your local hardware via ONNX Runtime. There are no API keys, no usage fees, and no data ever leaves your machine.

### Will the free tier ever be limited?

No. The local OSS core will never be paywalled. Cloud features are additive, not gating.

---

## Support

### Community (Free)

- GitHub Issues: [Report bugs or request features](https://github.com/steeltroops-ai/omnicontext/issues)
- GitHub Discussions: [Ask questions](https://github.com/steeltroops-ai/omnicontext/discussions)
- Documentation: [omnicontext.dev/docs](https://omnicontext.dev/docs)

### Enterprise (Paid)

- Email: [enterprise@omnicontext.dev](mailto:enterprise@omnicontext.dev)
- Priority response SLA
- Custom deployment assistance

---

## License

OmniContext is licensed under the [Apache License 2.0](https://github.com/steeltroops-ai/omnicontext/blob/main/LICENSE).

```
Copyright (c) 2024–2026 OmniContext Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0
```
