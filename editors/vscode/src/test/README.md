# OmniContext Extension Tests

This directory contains unit and integration tests for the OmniContext VS Code extension.

## Running Tests

### Prerequisites

Install test dependencies:
```bash
cd editors/vscode
npm install
# or
bun install
```

### Run All Tests

```bash
npm test
# or
bun test
```

This will:
1. Compile the TypeScript code
2. Download VS Code test instance (if not already downloaded)
3. Run all tests in the `test/` directory

### Run Tests in Watch Mode

```bash
npm run watch
```

Then in another terminal:
```bash
npm test
```

## Test Structure

- `test/suite/index.ts` - Test suite loader (discovers and runs all `*.test.js` files)
- `test/runTest.ts` - Test runner (launches VS Code test instance)
- `test/symbolExtractor.test.ts` - Unit tests for SymbolExtractor

## Writing Tests

Tests use the Mocha test framework with TDD-style syntax:

```typescript
import * as assert from 'assert';
import * as vscode from 'vscode';

suite('My Test Suite', () => {
    setup(() => {
        // Runs before each test
    });

    teardown(() => {
        // Runs after each test
    });

    test('should do something', async () => {
        // Test code
        assert.strictEqual(1 + 1, 2);
    });
});
```

## Test Coverage

Current test coverage:
- ✅ SymbolExtractor: Comprehensive unit tests
- ⏳ EventTracker: Pending (Task 6.1)
- ⏳ IPC Integration: Pending (Task 6.3)

## Debugging Tests

1. Open VS Code
2. Open the extension project
3. Set breakpoints in test files
4. Press F5 or use "Run > Start Debugging"
5. Select "Extension Tests" launch configuration

## CI/CD

Tests are automatically run in CI on:
- Pull requests
- Commits to main branch

## Troubleshooting

### Tests fail to start

- Ensure VS Code test instance can be downloaded
- Check internet connection
- Try deleting `.vscode-test` directory and re-running

### Tests timeout

- Increase timeout in `test/suite/index.ts`
- Check for async operations without proper awaits

### Symbol provider tests fail

- Some tests depend on VS Code's language features
- Ensure test documents use supported languages (typescript, javascript)
- Fallback to word extraction should always work
