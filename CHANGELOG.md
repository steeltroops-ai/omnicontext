# Changelog

All notable changes to OmniContext are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-03-14

### Added
- Pipeline, search, index, graph, parser, watcher improvements ([a672dc3](https://github.com/steeltroops-ai/omnicontext/commit/a672dc3dec8e91983d3b080b3f6f7f2e4187f872))
- Expand tool registry to 29 tools; add SSE transport ([e89ea5d](https://github.com/steeltroops-ai/omnicontext/commit/e89ea5d0f741693358f63cd645ce4cc4042dad65))
- Add SCIP interchange, rules injection, and memory store ([0fcd88b](https://github.com/steeltroops-ai/omnicontext/commit/0fcd88b9dd7c2dee6c1e151956b7cdfd0e3467ac))
- Replace jina with CodeRankEmbed; add cloud embedder ([db75b9a](https://github.com/steeltroops-ai/omnicontext/commit/db75b9a5d1e9399b72d4850aaf7cbaec86f28451))
- Add reranker model download step to install scripts ([d29b17f](https://github.com/steeltroops-ai/omnicontext/commit/d29b17f30bc9aa1e2466681f6722c2f7713f6ebd))
- Add reranker-download setup subcommand ([2b471da](https://github.com/steeltroops-ai/omnicontext/commit/2b471da040a1bb2a4fe01dbeba154fed30e980e8))

### Fixed
- Migrate model surface from jina to CodeRankEmbed ([b7a1162](https://github.com/steeltroops-ai/omnicontext/commit/b7a11629647845ef7965dfa0ba697658ad9a19e6))

## [1.2.3] - 2026-03-13

### Fixed
- Restore audit-check continue-on-error; supply-chain is authoritative gate ([98301bd](https://github.com/steeltroops-ai/omnicontext/commit/98301bd3299e1d1dcd1742629d5f6aae6eb040da))

## [1.2.2] - 2026-03-13

### Documentation
- Fill extension changelog gaps for versions 0.16.0 through 1.2.1 ([d592c4e](https://github.com/steeltroops-ai/omnicontext/commit/d592c4e802ac861ecc7125c059c317ad021ea0b4))
- Rewrite architecture intelligence docs to production standard and fix all version/count discrepancies ([a637a23](https://github.com/steeltroops-ai/omnicontext/commit/a637a231bf4cde59a8c57d847da53b1511e823dc))
- Add CLAUDE.md — consolidated agent operational guide ([f784287](https://github.com/steeltroops-ai/omnicontext/commit/f784287a284bfc167d5bb2e061249d6e47dfc0d4))
- Rewrite changelogs to enterprise standards and fix cliff.toml ([629e07a](https://github.com/steeltroops-ai/omnicontext/commit/629e07aa200b322eff647c537a4fc91b40db9bd6))

### Fixed
- Implement SCC cycle detection, fix extension context bug, remove phase labels ([fdc0033](https://github.com/steeltroops-ai/omnicontext/commit/fdc0033774ff09f28034c2d8542916cd547029df))
- Track and expose last index timestamp in system_status IPC handler ([c9f01cb](https://github.com/steeltroops-ai/omnicontext/commit/c9f01cb9e7438691545da7ec7ebdf894083bc7c7))
- Use cargo set-version in release workflow; remove audit continue-on-error ([eef7d9f](https://github.com/steeltroops-ai/omnicontext/commit/eef7d9f1af51deebaffc21b32006426e8a42bab8))
- Normalize file extensions to lowercase before language detection ([f0f8459](https://github.com/steeltroops-ai/omnicontext/commit/f0f8459f66351e3fc6077971829bb238b5496573))

## [1.2.1] - 2026-03-12

### Fixed
- Restore distribution enhancements stripped by release automation ([e28640f](https://github.com/steeltroops-ai/omnicontext/commit/e28640f3f893e073c8f8550870371a92caea76a6))
- Use platform config variables to avoid unused-variable errors on Linux ([116c6ca](https://github.com/steeltroops-ai/omnicontext/commit/116c6ca3e0e139093a21be75f548e992ad79889c))

## [1.2.0] - 2026-03-12

### Added
- Add Antigravity IDE support, harden distribution scripts, and enhance MCP tools ([99dd59f](https://github.com/steeltroops-ai/omnicontext/commit/99dd59fa2d87d491d22cbc5fd64308b40cd612f1))
- Improve indexed repo visibility and repo actions ([6645aa4](https://github.com/steeltroops-ai/omnicontext/commit/6645aa496f5e8040aaafda9ccad702957c923f94))

### Fixed
- Mark ONNX-dependent FFI tests as #[ignore] ([d9f483e](https://github.com/steeltroops-ai/omnicontext/commit/d9f483e1158ca4b3e550e3a37e56caee53e611bc))

## [1.1.2] - 2026-03-11

### Fixed
- Harden index pipeline and extension daemon lifecycle ([57b67e5](https://github.com/steeltroops-ai/omnicontext/commit/57b67e566abfd76984da5b09d2b254e458d5a01a))

## [1.1.1] - 2026-03-09

### Fixed
- Resolve 404 errors and mermaid rendering issues ([ba3da03](https://github.com/steeltroops-ai/omnicontext/commit/ba3da03a6c49bb9215a49ee74d4cce23e260e898))

## [1.1.0] - 2026-03-09

### Added
- Add focus-based mermaid diagram controls ([a09d625](https://github.com/steeltroops-ai/omnicontext/commit/a09d62575d42b7dfe05ca2a450380dee3246b8db))
- Add pan and mouse wheel zoom to mermaid diagrams ([6683e69](https://github.com/steeltroops-ai/omnicontext/commit/6683e69614b49742efefe3f216a8901d5b8ad1c9))
- Add mermaid diagram support with interactive architecture documentation ([03448db](https://github.com/steeltroops-ai/omnicontext/commit/03448db097c0d651221a8bc79a299cdaa7b3bdbd))
- Implement markdown-based documentation system ([c0c65b5](https://github.com/steeltroops-ai/omnicontext/commit/c0c65b5cbcb3c132e159689a777d35a7162c03db))
- Implement smooth scrolling with lenis ([2d90438](https://github.com/steeltroops-ai/omnicontext/commit/2d9043873efc3ea7c0c93ba4818a77c647cdd952))

### Documentation
- Add architecture, configuration, contributing, and enterprise pages ([94105dd](https://github.com/steeltroops-ai/omnicontext/commit/94105ddac9c72835d3149870fb6d9ef649c29b0f))
- Create professional documentation with proper naming conventions ([109d5fb](https://github.com/steeltroops-ai/omnicontext/commit/109d5fbe42b951bfca4e098fe8d1c1af532f5a88))
- Create minimal essential documentation for website ([53c2899](https://github.com/steeltroops-ai/omnicontext/commit/53c2899b41dba439efd284b39ebd7bd49bb5930a))
- Create production documentation structure and phase 1 mvp pages ([bd60f45](https://github.com/steeltroops-ai/omnicontext/commit/bd60f450ee8172605662885e42c3903aebf34d1f))
- Add comprehensive end-to-end feature audit and improvement roadmap ([674c18b](https://github.com/steeltroops-ai/omnicontext/commit/674c18b56c0ef669a3996f5d43cb9c48b260ca55))

### Fixed
- Resolve hydration errors with stable ID generation ([4d440a7](https://github.com/steeltroops-ai/omnicontext/commit/4d440a7f8601e9342ed816a3d5c391ff9968546c))
- Implement industry-standard controls for mermaid diagrams ([0eedb68](https://github.com/steeltroops-ai/omnicontext/commit/0eedb6870d8af6f2dc993c4c51e465c65256ae79))
- Correct docs path to resolve sidebar not showing documentation ([d129b9d](https://github.com/steeltroops-ai/omnicontext/commit/d129b9d1b6ff0770fd6e57ef02fcbdb6fea2df76))
- Add scroll spy to table of contents and show all docs in sidebar ([b5221fe](https://github.com/steeltroops-ai/omnicontext/commit/b5221fedab971364ca55741430a735b2cb57a9d3))
- Restore original introduction content and add table of contents ([9a05f85](https://github.com/steeltroops-ai/omnicontext/commit/9a05f85c22e11792fa3fcaca04152d8667687bff))
- Resolve markdown rendering and sidebar organization issues ([4bbba0f](https://github.com/steeltroops-ai/omnicontext/commit/4bbba0f170b7d145424ddcf9321469c08801e697))
- Add static pages to sidebar navigation ([f0634cc](https://github.com/steeltroops-ai/omnicontext/commit/f0634cc49c1de6ca0478d7f168f9351f7e53db34))
- Correct docs path to parent directory ([9005dd3](https://github.com/steeltroops-ai/omnicontext/commit/9005dd3ba8dde5b60bf43bd2eba232bd88efe649))
- Correct context engine visualization labels ([2e3ffc4](https://github.com/steeltroops-ai/omnicontext/commit/2e3ffc48faeddd0733efd203856143903866b7cc))
- Correct mcp tools count from 8 to 16 ([95abeed](https://github.com/steeltroops-ai/omnicontext/commit/95abeed9c8a2f3ab52e9ad19de5148abfceb38e9))
- Make toc sidebar fixed position on docs pages ([2a348d1](https://github.com/steeltroops-ai/omnicontext/commit/2a348d1fbc97107b2e82b40cbfdbc252a46e01ea))
- Enable lenis smooth scrolling on docs pages ([93b6e15](https://github.com/steeltroops-ai/omnicontext/commit/93b6e15bb794072dd958e39c8e03bb63b3bd3291))
- Enable full-page screenshots by using document-level scroll ([7fb2696](https://github.com/steeltroops-ai/omnicontext/commit/7fb269634a2998bb0d13b9f0f38c691dbc99dbf3))
- Improve footer and context engine section responsiveness ([69d0e9e](https://github.com/steeltroops-ai/omnicontext/commit/69d0e9e0b16544dae2e22411f55213f89391739a))

## [1.0.1] - 2026-03-09

### Fixed
- Align changelog versions with git tags and fix vscode engines compatibility ([f22c2e0](https://github.com/steeltroops-ai/omnicontext/commit/f22c2e02582ee2b646363ced305bb07ccdd8fccf))

## [0.16.1] - 2026-03-09

### Fixed
- Resolve broken doctests and async runtime issues ([c5380e8](https://github.com/steeltroops-ai/omnicontext/commit/c5380e88da8079d47ea43d9132158c12235866c3))
- Resolve compilation errors in module exports and type annotations ([5ddfc63](https://github.com/steeltroops-ai/omnicontext/commit/5ddfc6339d6a8d1035e0e91a58508a36c42de222))

## [0.16.0] - 2026-03-09

### Added
- Add connection pooling for concurrent database access ([9a8ae9e](https://github.com/steeltroops-ai/omnicontext/commit/9a8ae9eb1817860b1b74f046832afeeb7962982c))
- Add contextual chunking and query result caching ([d668215](https://github.com/steeltroops-ai/omnicontext/commit/d66821504b4f6477b4b63888d8a5f8f694f3ad28))
- Add batching, contrastive learning, and quantization support ([b18fce6](https://github.com/steeltroops-ai/omnicontext/commit/b18fce6102be1505a5afa40149b8853176641b2e))

### Documentation
- Restructure documentation with comprehensive guides ([97d8db9](https://github.com/steeltroops-ai/omnicontext/commit/97d8db96878ee01041940d7aa2fdcaf32c23fb1d))

### Testing
- Add benchmarks and golden query test suite ([8a9dd68](https://github.com/steeltroops-ai/omnicontext/commit/8a9dd68248517641747d2f81540af95d6b26593f))

## [0.15.0] - 2026-03-09

### Added
- Add resilience monitoring and file dependency infrastructure ([f8ea24a](https://github.com/steeltroops-ai/omnicontext/commit/f8ea24a4db47e66e53979f61cea3f8ee5b330650))
- Add graph visualization and performance monitoring UI ([55293d7](https://github.com/steeltroops-ai/omnicontext/commit/55293d7ae2aaa7ac86a7c6372c7d8df924f36336))
- Add IPC handlers for VS Code extension phases 4-6 ([a25b9f8](https://github.com/steeltroops-ai/omnicontext/commit/a25b9f875271d82ec2466eb039b921dd0385b683))
- Add file-level dependency graph for architectural context ([841d4b5](https://github.com/steeltroops-ai/omnicontext/commit/841d4b52be0d9ad61d2f80ba3e0804fdf70806f3))

### Fixed
- Update embedder tests to use RERANKER_MODEL ([16c2bf6](https://github.com/steeltroops-ai/omnicontext/commit/16c2bf62df4b41040ac4629b5d9e27a510b6eb4a))

## [0.14.0] - 2026-03-08

### Added
- Implement branch-aware diff indexing and sota performance optimizations ([464ab1f](https://github.com/steeltroops-ai/omnicontext/commit/464ab1f02c79f8a8ad5593a463646f0f672c7f04))

## [0.13.1] - 2026-03-08

### Fixed
- Harden path resolution to prevent silent wrong-dir indexing ([38ad9a0](https://github.com/steeltroops-ai/omnicontext/commit/38ad9a018ec25c4499d63bc16aa90eb109ebc034))
- Resolve onnx runtime version mismatch dynamically ([8943d04](https://github.com/steeltroops-ai/omnicontext/commit/8943d042ecd276913456c8d7537592e73d2f3b93))

## [0.13.0] - 2026-03-08

### Added
- Implement batch embedding and backpressure for indexing ([ea880bf](https://github.com/steeltroops-ai/omnicontext/commit/ea880bfb16061dd9b5e53a4ee0444f20f5dff687))

## [0.11.0] - 2026-03-07

### Added
- Overhaul sidebar UI, fix path normalization, and add set_workspace tool ([4a8cf35](https://github.com/steeltroops-ai/omnicontext/commit/4a8cf356288466a211ac9187d8910f5fa85b1f4c))

### Documentation
- Add marketplace badges and pre-rendered mermaid diagrams to readmes ([1b38992](https://github.com/steeltroops-ai/omnicontext/commit/1b389925854a04422f335c7dad75f2e285ba44f0))

## [0.10.0] - 2026-03-07

### Added
- Core mcp and daemon optimizations ([f4f4450](https://github.com/steeltroops-ai/omnicontext/commit/f4f445096d3bdf2aa3837283dac592d79d8f8fbd))

## [0.9.4] - 2026-03-07

### Documentation
- Clean up unused markdown link references in vscode changelog ([f608990](https://github.com/steeltroops-ai/omnicontext/commit/f6089906f8fad42edda9f12f4d79bbb29784f2ae))
- Restructure vscode changelog to correctly group 0.9.x features under 0.9.2 and restore Unreleased header ([ea23b10](https://github.com/steeltroops-ai/omnicontext/commit/ea23b107cbf32ce6538747e3b951921ce476e686))

### Fixed
- Generate separate scoped changelog for vscode extension ([1f29da2](https://github.com/steeltroops-ai/omnicontext/commit/1f29da28de13f345581dbe7fea6ba2cea54db230))

## [0.9.3] - 2026-03-07

### Fixed
- Resolve release workflow failures and rewrite changelog generation ([85161e3](https://github.com/steeltroops-ai/omnicontext/commit/85161e32a1c3c725d6ce09ebe6d37930fedc0066))
- Resolve all workflow failures - license allowlist, security gate logic, release archive, changelog output ([f38e62f](https://github.com/steeltroops-ai/omnicontext/commit/f38e62f5b2dc4fa065bb168143b75953a6dcf425))

## [0.9.2] - 2026-03-07

### Fixed
- Remove invalid default key from deny.toml licenses section to resolve cargo-deny parsing error ([b58841f](https://github.com/steeltroops-ai/omnicontext/commit/b58841f65b05133df972c11e4f2e960a21aa667b))

## [0.9.1] - 2026-03-07

### Documentation
- Restructure and normalize documentation to kebab-case naming conventions ([0938a16](https://github.com/steeltroops-ai/omnicontext/commit/0938a160fd57c1e4506840cae800d99cbe024c05))

### Fixed
- Migrate deny.toml to cargo-deny v2, eliminate ort mutex poisoning in tests ([00f2478](https://github.com/steeltroops-ai/omnicontext/commit/00f24785a8e5174cc2038d4ea5b17ab244bb2885))

## [0.9.0] - 2026-03-07

### Added
- Zero-friction bootstrap, ONNX auto-install, sidebar circuit breaker ([69d1612](https://github.com/steeltroops-ai/omnicontext/commit/69d1612197816a8eaa5332a4395f6f723ed6021f))

## [0.8.0] - 2026-03-07

### Added
- Implement managed setup command and unify premium UX across distribution scripts ([1008141](https://github.com/steeltroops-ai/omnicontext/commit/1008141ecaa6e072d4350b2ca122824458906e05))

## [0.7.1] - 2026-03-06

### Fixed
- Invert version resolution to github releases API, remove unicode em-dash ([9b8bb7e](https://github.com/steeltroops-ai/omnicontext/commit/9b8bb7ef36b916bceb7069b49ecaf31e2ad89a28))

## [0.7.0] - 2026-03-06

### Added
- Overhaul distribution UX and cross-platform IDE support ([98ac2c5](https://github.com/steeltroops-ai/omnicontext/commit/98ac2c58965f57182ff06dc106e50b061de10ef6))

## [0.6.1] - 2026-03-06

### Fixed
- Add write permission to build job for release asset upload ([71351fb](https://github.com/steeltroops-ai/omnicontext/commit/71351fb9b5e2192bc08bd77480f7b2fa171bfdd0))

## [0.6.0] - 2026-03-06

### Added
- Complete Zero-Config MCP architecture and manifest publishing pipeline ([4cfb0f4](https://github.com/steeltroops-ai/omnicontext/commit/4cfb0f4cc2589eb7320fb5cff540884dda497ca4))
- Implement intelligence architecture enhancements ([b4f46e1](https://github.com/steeltroops-ai/omnicontext/commit/b4f46e14530959f447404a0e318f5103499c8c25))

## [0.5.3] - 2026-03-02

### Fixed
- Update MCP install/test scripts ([da0c123](https://github.com/steeltroops-ai/omnicontext/commit/da0c1237d4f1b13f9ba3d3ce1abf9258fecd421c))

## [0.5.2] - 2026-03-02

### Fixed
- Skip release build jobs when no release needed ([16db7f7](https://github.com/steeltroops-ai/omnicontext/commit/16db7f7ff3c32360b94c0305c9cf5c18e38b58e4))

## [0.5.1] - 2026-03-02

### Fixed
- Correct deny.toml configuration values ([0c808d5](https://github.com/steeltroops-ai/omnicontext/commit/0c808d55a36f5908b183eb7e5453ee254ff7a371))

## [0.5.0] - 2026-03-02

### Added
- Dynamic version detection from source ([673ad51](https://github.com/steeltroops-ai/omnicontext/commit/673ad511953887fb63472da60481c6e519770469))

## [0.4.0] - 2026-03-02

### Added
- Complete Phase 7 enhanced sidebar UI with professional codicons ([45a3d87](https://github.com/steeltroops-ai/omnicontext/commit/45a3d8758fe83ec7eb4dc6fb181076ca0f62ac98))
- Expose language distribution in engine status ([c768416](https://github.com/steeltroops-ai/omnicontext/commit/c768416f68db1d2269805570069f4acec2c58031))

### Documentation
- Add comprehensive install/update/uninstall commands for all platforms ([c486857](https://github.com/steeltroops-ai/omnicontext/commit/c48685796f7c028bcddcfb0198f88d9f4970cd50))

### Fixed
- Prevent database lock conflicts in concurrent tests ([a520a9a](https://github.com/steeltroops-ai/omnicontext/commit/a520a9a69efb50d8685a6dcfdcb5d381b8abe3b5))
- Correct install/update command paths in README ([f0b426d](https://github.com/steeltroops-ai/omnicontext/commit/f0b426d2730d73240b42ea2fbffea8c68fe5c2b2))

## [0.3.0] - 2026-03-01

### Added
- Add per-language file distribution to index engine ([bd0e1f9](https://github.com/steeltroops-ai/omnicontext/commit/bd0e1f9cff12044cdf88768acd4ab1fc298b0a16))

### Documentation
- Reorganize to enterprise-grade structure with Mermaid diagrams ([68fc248](https://github.com/steeltroops-ai/omnicontext/commit/68fc2482fc434a4495b75cbdfc0212fc5ba84537))
- Reorganize documentation into enterprise structure ([f0e7f2b](https://github.com/steeltroops-ai/omnicontext/commit/f0e7f2bdaffba664ada149dd1a1f0b1fd06a5bdb))
- Reorganize into professional enterprise structure ([1a7b3af](https://github.com/steeltroops-ai/omnicontext/commit/1a7b3afbaad218b59c95ef2ae968f7b62a31c625))

## [0.2.0] - 2026-03-01

### Added
- Add automatic version bumping with conventional commits ([e922d45](https://github.com/steeltroops-ai/omnicontext/commit/e922d4569f76cadc356b4e62b65b7f45f0ba3872))
- Add automatic version bumping with conventional commits ([01c30c5](https://github.com/steeltroops-ai/omnicontext/commit/01c30c5946252f729119e7b5b94cba87763c8517))
- Add enterprise-grade workflows and version bumping ([5cf7b1d](https://github.com/steeltroops-ai/omnicontext/commit/5cf7b1dfcd7bf1cc80a203912e93befacb373af7))
- Add comprehensive code quality guardrails ([7abe484](https://github.com/steeltroops-ai/omnicontext/commit/7abe48436b3f9c227aea523407803ffc25a18b56))
- Add project organization steering rules and clean up repository ([31f3004](https://github.com/steeltroops-ai/omnicontext/commit/31f30041f61f1644ffcf219da7226a824677d892))
- Add enterprise-grade installation and testing scripts ([14ac869](https://github.com/steeltroops-ai/omnicontext/commit/14ac8693a6a000ab7956374fa77911f78b0b24be))
- Fix critical gaps - embedding coverage, graph loading, and benchmark suite ([07f266f](https://github.com/steeltroops-ai/omnicontext/commit/07f266f6a2fe4e4b3d7d88ba0137d0a74598f0ab))
- Implement CAST micro-chunking overlap and cross-encoder reranking pipeline ([da9bbfc](https://github.com/steeltroops-ai/omnicontext/commit/da9bbfc6a88a648ed48ee26d6856f4c6468c8d94))
- Implement knowledge graph phase 2 and context assembly phase 3 ([88ce33c](https://github.com/steeltroops-ai/omnicontext/commit/88ce33c0e5a2a284804490f3c851042616c36ce4))
- Implement Contextual Enricher for chunks ([5b54fd7](https://github.com/steeltroops-ai/omnicontext/commit/5b54fd72569a069480fbbf2924a46b196f6a0543))
- Query expansion and result deduplication\n\n- expand queries to split code tokens (snake_case, CamelCase, etc) for better BM25 match\n- deduplicate overlapping chunks from the same file (keep highest score) ([04a15c1](https://github.com/steeltroops-ai/omnicontext/commit/04a15c170876143be610470b69c8b42fc2ea2049))
- Graph-boosted ranking with dependency proximity\n\n- integrate DependencyGraph into hybrid search for in-degree and proximity boosting\n- anchor top-3 results to find related symbols via graph distance\n- store relative paths in pipeline for cross-platform consistency\n- update gitignore to exclude onnx binaries" ([538ccc6](https://github.com/steeltroops-ai/omnicontext/commit/538ccc6a0c3223f84c64335bd524b58b485be482))
- Implement accurate module-qualified FQNs across all languages\n\n- added `build_module_name_from_path` to strip directories like `src/` and construct root-relative module paths\n- applied to Rust, TypeScript, Python, C++, C#, Java, Go, C, CSS, and Document analyzers\n- ensures `auth::user::MyStruct` instead of just `user::MyStruct` depending on repo hierarchy" ([b10cd28](https://github.com/steeltroops-ai/omnicontext/commit/b10cd280850775b55589a6ba839263f01447eb8d))
- Add Java, C, C++, C#, CSS analyzers and Markdown/TOML/YAML/JSON/HTML/Shell doc indexing\n\n- expand Language enum from 5 to 16 variants\n- add tree-sitter grammars for Java, C, C++, C#, CSS, Markdown\n- implement JavaAnalyzer with class/interface/method/Javadoc extraction\n- implement CAnalyzer with function/struct/macro/typedef extraction\n- implement CppAnalyzer with namespace/class/template support\n- implement CSharpAnalyzer with namespace/class/interface/property parsing\n- implement CssAnalyzer with rule set/media query extraction\n- implement DocumentAnalyzer for Markdown (heading-sectioned), TOML (table-sectioned), and generic formats\n- register all 16 analyzers in global registry\n- update watcher tests for expanded file type support" ([6170d67](https://github.com/steeltroops-ai/omnicontext/commit/6170d67faadb88a8d58f6de3952bffce3c43b17f))
- Implement phase 0+1 search quality and dependency graph fixes\n\n- add extract_imports for TS, JS, Go analyzers (dep graph was empty)\n- wire parse_imports into pipeline to populate dependency edges\n- add structural weight boost to search scoring (kind+visibility)\n- add query expansion for NL queries (stop-word stripping, OR join)\n- add get_first_symbol_for_file index helper for import resolution\n- fetch 2x candidates before structural re-ranking for better recall ([7863c85](https://github.com/steeltroops-ai/omnicontext/commit/7863c85e6af12ba90fbf8513287fccea27c71c6c))
- Add one-click install scripts for automatic bin + model download ([ae8e64d](https://github.com/steeltroops-ai/omnicontext/commit/ae8e64d88030cb3beb1584f301cd79a0804c8404))

### Changed
- Apply cargo fmt to fix formatting issues ([99cfe66](https://github.com/steeltroops-ai/omnicontext/commit/99cfe66af61115178f19cc8589d1e45dd5d3e75e))
- Reorganize installation scripts and fix workflows ([34d1098](https://github.com/steeltroops-ai/omnicontext/commit/34d109833278d9137942a54d8840334c962565e2))
- Major cleanup - organize project to enterprise standards ([5046b75](https://github.com/steeltroops-ai/omnicontext/commit/5046b75f91d188d36a93c0c4371365bbd74946a4))

### Documentation
- Comprehensive project cleanup and organization ([310cb0d](https://github.com/steeltroops-ai/omnicontext/commit/310cb0dbde6bb4591318c94d23dca269d20d8782))

### Fixed
- Resolve all clippy warnings in main crates ([c0a8264](https://github.com/steeltroops-ai/omnicontext/commit/c0a826430f1ccd69fb14085540810d0d2f3558d3))
- Update tests binary path and improve pipeline telemetry calculations ([907eaae](https://github.com/steeltroops-ai/omnicontext/commit/907eaaea3c44c334027d3ca621f5b27868f0e277))
- Improve MCP server test reliability ([37a82a4](https://github.com/steeltroops-ai/omnicontext/commit/37a82a4e3033c2ecc89e02be87b607b9387b61d5))
- Guarantee 100% chunk coverage on ONNX partial failures\n\n- modify `embed_batch` to return `Vec<Option<Vec<f32>>>` instead of failing the file\n- fallback to single-chunk embedding if a batched inference fails (e.g., ONNX error for an oversized chunk)\n- gracefully store successfully encoded chunks while leaving failed chunks with keyword-only retrieval (vs dropping all embeddings for the file)" ([c7ccda5](https://github.com/steeltroops-ai/omnicontext/commit/c7ccda53505a6079f4952f48fb7bd3d248701067))
- Resolve get_file_summary path normalization for Windows UNC paths\n\n- expose Engine::repo_path() for MCP tools\n- try multiple path variants: relative, absolute, UNC-stripped, canonicalized\n- provide actionable error message on file-not-found" ([4575d9c](https://github.com/steeltroops-ai/omnicontext/commit/4575d9c05bbfd2ebe5f68dfea9da5fba9e380589))

<!-- generated by git-cliff -->
