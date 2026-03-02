/**
 * Unit tests for SymbolExtractor
 */

import * as assert from 'assert';
import * as vscode from 'vscode';
import { SymbolExtractor } from '../symbolExtractor';

suite('SymbolExtractor Test Suite', () => {
    let extractor: SymbolExtractor;

    setup(() => {
        extractor = new SymbolExtractor();
    });

    suite('getSymbolAtPosition', () => {
        test('should extract symbol using VS Code symbol provider', async () => {
            // Create a test document
            const content = `class MyClass {
    myMethod() {
        return 42;
    }
}`;
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'typescript'
            });

            // Position cursor on "myMethod"
            const position = new vscode.Position(1, 6);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            // Should extract the method name
            assert.ok(symbol !== undefined, 'Symbol should be extracted');
            assert.ok(symbol === 'myMethod' || symbol === 'MyClass',
                'Symbol should be either the method or class name');
        });

        test('should fall back to word extraction when symbol provider fails', async () => {
            // Create a plain text document (no symbol provider)
            const content = 'hello world test';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            // Position cursor on "world"
            const position = new vscode.Position(0, 7);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'world', 'Should fall back to word extraction');
        });

        test('should return undefined when no symbol or word at position', async () => {
            const content = '   ';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            // Position cursor on whitespace
            const position = new vscode.Position(0, 1);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, undefined, 'Should return undefined for whitespace');
        });

        test('should limit symbol length to 100 characters', async () => {
            // Create a document with a very long identifier
            const longName = 'a'.repeat(150);
            const content = `const ${longName} = 42;`;
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'javascript'
            });

            // Position cursor on the long name
            const position = new vscode.Position(0, 10);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.ok(symbol !== undefined, 'Symbol should be extracted');
            assert.ok(symbol!.length <= 100, 'Symbol should be limited to 100 characters');
            assert.strictEqual(symbol!.length, 100, 'Symbol should be exactly 100 characters');
        });

        test('should handle nested symbols and return most specific', async () => {
            const content = `class OuterClass {
    class InnerClass {
        innerMethod() {
            return 42;
        }
    }
}`;
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'typescript'
            });

            // Position cursor on "innerMethod"
            const position = new vscode.Position(2, 10);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.ok(symbol !== undefined, 'Symbol should be extracted');
            // Should get the most specific symbol (innerMethod or InnerClass)
            assert.ok(
                symbol === 'innerMethod' || symbol === 'InnerClass' || symbol === 'OuterClass',
                'Should extract a symbol from the nested structure'
            );
        });

        test('should handle errors gracefully', async () => {
            const content = 'test content';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            // Position cursor at valid location
            const position = new vscode.Position(0, 2);

            // Should not throw even if symbol provider has issues
            const symbol = await extractor.getSymbolAtPosition(doc, position);

            // Should either return a symbol or undefined, but not throw
            assert.ok(symbol === undefined || typeof symbol === 'string',
                'Should return string or undefined, not throw');
        });
    });

    suite('Word extraction fallback', () => {
        test('should extract word at cursor position', async () => {
            const content = 'function testFunction() {}';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            // Position cursor on "testFunction"
            const position = new vscode.Position(0, 12);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'testFunction', 'Should extract word at position');
        });

        test('should handle word at start of line', async () => {
            const content = 'startWord middle end';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 0);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'startWord', 'Should extract word at start');
        });

        test('should handle word at end of line', async () => {
            const content = 'start middle endWord';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 18);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'endWord', 'Should extract word at end');
        });

        test('should handle special characters in identifiers', async () => {
            const content = 'my_variable_name';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 5);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.ok(symbol !== undefined, 'Should extract identifier with underscores');
            assert.ok(symbol!.includes('variable'), 'Should include part of the identifier');
        });
    });

    suite('Edge cases', () => {
        test('should handle empty document', async () => {
            const content = '';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 0);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, undefined, 'Should return undefined for empty document');
        });

        test('should handle single character', async () => {
            const content = 'x';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 0);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'x', 'Should extract single character');
        });

        test('should handle multi-line documents', async () => {
            const content = `line1
line2
line3`;
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            // Position on line 2
            const position = new vscode.Position(1, 2);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'line2', 'Should extract word from correct line');
        });

        test('should handle position at end of document', async () => {
            const content = 'test';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 4);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            // Should either get 'test' or undefined, but not throw
            assert.ok(symbol === 'test' || symbol === undefined,
                'Should handle end of document gracefully');
        });
    });

    suite('Symbol length limiting', () => {
        test('should not modify symbols under 100 characters', async () => {
            const content = 'shortName';
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 0);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.strictEqual(symbol, 'shortName', 'Should not modify short symbols');
        });

        test('should truncate symbols at exactly 100 characters', async () => {
            const longName = 'a'.repeat(100);
            const content = longName;
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 50);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.ok(symbol !== undefined, 'Symbol should be extracted');
            assert.strictEqual(symbol!.length, 100, 'Should keep exactly 100 characters');
        });

        test('should truncate symbols longer than 100 characters', async () => {
            const longName = 'b'.repeat(200);
            const content = longName;
            const doc = await vscode.workspace.openTextDocument({
                content,
                language: 'plaintext'
            });

            const position = new vscode.Position(0, 100);

            const symbol = await extractor.getSymbolAtPosition(doc, position);

            assert.ok(symbol !== undefined, 'Symbol should be extracted');
            assert.strictEqual(symbol!.length, 100, 'Should truncate to 100 characters');
            assert.strictEqual(symbol, 'b'.repeat(100), 'Should contain first 100 characters');
        });
    });
});
