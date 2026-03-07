# OmniContext Enterprise License Matrix

Copyright (c) 2026, Mayank. All rights reserved.

This document clearly delineates the legal operating parameters and licensing architecture for the OmniContext Codebase Engine and its Enterprise extensions.

## Dual-License Architecture

| Component Boundary               | Governance License         | Allowed Usage                                 | Restrictions                    |
| :------------------------------- | :------------------------- | :-------------------------------------------- | :------------------------------ |
| **`omni-core`** (Core Engine)    | Apache 2.0                 | Open-source, personal, commercial derivation. | Must preserve copyright notice. |
| **`omni-mcp`** (Daemon)          | Apache 2.0                 | Open-source, personal, commercial derivation. | Must preserve copyright notice. |
| **`omni-cli`** (Terminal Client) | Apache 2.0                 | Open-source, personal, commercial derivation. | Must preserve copyright notice. |
| **`omni-api`** (Headless Routes) | **Proprietary Commercial** | Internal evaluation.                          | **NO PRODUCTION USAGE**.        |
| **Enterprise RBAC Plugins**      | **Proprietary Commercial** | Internal evaluation.                          | **NO PRODUCTION USAGE**.        |

## Commercial Software Boundaries (Proprietary Tier)

Components explicitly marked under this Proprietary Commercial License are strictly governed by the following operational constraints.

### 1. Permitted Uses

You are explicitly permitted to:

1. Compile the proprietary source boundaries locally.
2. Read, audit, and evaluate the proprietary component logic.
3. Execute the service exclusively in non-production, air-gapped test architectures.

### 2. Immediate Restrictions

Without a verified Subscription or Commercial Agreement executed with Mayank, you and your organization are strictly prohibited from:

- **(a) Derivation:** Modifying, translating, or reverse-engineering proprietary modules.
- **(b) Production Traffic:** Deploying the software inside any continuous integration, staging, or live production environment servicing users or LLM agent traffic.
- **(c) Sub-licensing:** Renting, leasing, selling, or transferring copies of the software to any third-party entity.
- **(d) Service Bureau / SaaS:** Using the proprietary backend to operate a remote indexing service for external clients.
- **(e) Obfuscation:** Stripping or altering copyright headers embedded in the source code.

## Liability & Warranty

THE PROPRIETARY SOFTWARE IS SUPPLIED "AS IS", STRICTLY EXCLUDING ALL WARRANTIES OF MERCHANTABILITY OR FITNESS FOR A PARTICULAR COMPUTATIONAL ENVIRONMENT.

IN NO EVENT SHALL THE COPYRIGHT HOLDER BE LEGALLY LIABLE FOR CORRUPTION OF REPOSITORY CONTEXT, VECTOR DATA LOSS, OR ANY OTHER HARDWARE DEGRADATION ARISING DIRECTLY OR INDIRECTLY FROM THIS COMPONENT.

> **Commercial Authorization:** To bypass Section 2 restrictions and deploy enterprise multi-tenant architecture to production environments, initiate a commercial request via <steeltroops.ai@gmail.com>.
