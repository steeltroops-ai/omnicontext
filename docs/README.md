# OmniContext Documentation

Welcome to the OmniContext documentation. This directory contains comprehensive technical documentation for developers, contributors, and users.

## 📚 Documentation Structure

### 🏗️ Architecture (`architecture/`)

System design, architectural decisions, and technical specifications.

- **[ADR.md](architecture/ADR.md)** - Architecture Decision Records
- **[CONCURRENCY_ARCHITECTURE.md](architecture/CONCURRENCY_ARCHITECTURE.md)** - Concurrency patterns and thread safety
- **[SECURITY_THREAT_MODEL.md](architecture/SECURITY_THREAT_MODEL.md)** - Security analysis and threat mitigation

### 🔧 Development (`development/`)

Guidelines and strategies for contributors and maintainers.

- **[TESTING_STRATEGY.md](development/TESTING_STRATEGY.md)** - Testing approach and best practices
- **[ERROR_RECOVERY.md](development/ERROR_RECOVERY.md)** - Error handling patterns

### 📖 Guides (`guides/`)

User-facing guides and how-to documentation.

- **[CONVENTIONAL_COMMITS.md](guides/CONVENTIONAL_COMMITS.md)** - Commit message format for automatic versioning
- **[INSTALLATION_WORKFLOW.md](guides/INSTALLATION_WORKFLOW.md)** - Installation and setup guide
- **[SUPPORTED_LANGUAGES.md](guides/SUPPORTED_LANGUAGES.md)** - Programming languages supported by OmniContext

### 🔌 API (`api/`)

API documentation and integration guides (coming soon).

## 🚀 Quick Links

### For Users
- [Installation Guide](guides/INSTALLATION_WORKFLOW.md)
- [Supported Languages](guides/SUPPORTED_LANGUAGES.md)
- [Main README](../README.md)

### For Contributors
- [Contributing Guide](../CONTRIBUTING.md)
- [Conventional Commits](guides/CONVENTIONAL_COMMITS.md)
- [Testing Strategy](development/TESTING_STRATEGY.md)
- [GitHub Workflows](../.github/WORKFLOWS.md)

### For Architects
- [Architecture Decisions](architecture/ADR.md)
- [Concurrency Model](architecture/CONCURRENCY_ARCHITECTURE.md)
- [Security Model](architecture/SECURITY_THREAT_MODEL.md)

## 📝 Documentation Standards

### File Naming
- Use `SCREAMING_SNAKE_CASE.md` for major documents
- Use `kebab-case.md` for specific guides
- Keep names descriptive and searchable

### Content Guidelines
- Start with a clear title and purpose
- Include table of contents for long documents
- Use code examples where applicable
- Keep language clear and concise
- Update dates when making significant changes

### Organization
- **Architecture**: System design, patterns, decisions
- **Development**: Contributor workflows, testing, debugging
- **Guides**: User-facing how-to documentation
- **API**: Integration guides, protocol specs

## 🔄 Keeping Docs Updated

Documentation should be updated when:
- Architecture changes significantly
- New features are added
- APIs change
- Security considerations evolve
- Testing strategies are refined

Use conventional commits for doc updates:
```bash
git commit -m "docs: update concurrency architecture guide"
git commit -m "docs(api): add MCP tool reference"
```

## 🤝 Contributing to Docs

1. Check if documentation already exists
2. Place in appropriate directory
3. Follow naming conventions
4. Update this README if adding new sections
5. Use clear, professional language
6. Include code examples and diagrams

## 📧 Questions?

- Open an issue on GitHub
- Check existing documentation first
- Contribute improvements via pull requests

---

**Last Updated**: 2026-03-01  
**Maintained By**: OmniContext Team
