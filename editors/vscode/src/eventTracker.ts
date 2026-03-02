/**
 * IDE event tracking with debouncing and queue management.
 * Tracks file opens, cursor movements, and text edits, sending them to the daemon for pre-fetch.
 */

import * as vscode from 'vscode';
import { IdeEvent, EventTrackerConfig } from './types';
import { SymbolExtractor } from './symbolExtractor';

// Simple debounce implementation (avoiding external dependency for now)
type DebouncedFunc<T extends (...args: any[]) => any> = {
    (...args: Parameters<T>): void;
    cancel(): void;
};

function debounce<T extends (...args: any[]) => any>(
    func: T,
    wait: number
): DebouncedFunc<T> {
    let timeout: NodeJS.Timeout | null = null;

    const debounced = (...args: Parameters<T>) => {
        if (timeout) {
            clearTimeout(timeout);
        }
        timeout = setTimeout(() => {
            timeout = null;
            func(...args);
        }, wait);
    };

    debounced.cancel = () => {
        if (timeout) {
            clearTimeout(timeout);
            timeout = null;
        }
    };

    return debounced;
}

export class EventTracker {
    private eventQueue: IdeEvent[] = [];
    private enabled: boolean;
    private config: EventTrackerConfig;
    private disposables: vscode.Disposable[] = [];

    private cursorDebouncer: DebouncedFunc<
        (event: vscode.TextEditorSelectionChangeEvent) => void
    >;
    private editDebouncer: DebouncedFunc<
        (event: vscode.TextDocumentChangeEvent) => void
    >;

    constructor(
        private sendIpcRequest: (method: string, params: any) => Promise<any>,
        private symbolExtractor: SymbolExtractor,
        config: Partial<EventTrackerConfig> = {}
    ) {
        this.config = {
            enabled: config.enabled ?? true,
            debounceMs: config.debounceMs ?? 200,
            maxQueueSize: config.maxQueueSize ?? 100,
        };
        this.enabled = this.config.enabled;

        // Create debouncers
        this.cursorDebouncer = debounce(
            (event) => this.handleCursorMoved(event),
            this.config.debounceMs
        );

        this.editDebouncer = debounce(
            (event) => this.handleTextEdited(event),
            this.config.debounceMs
        );
    }

    /**
     * Register VS Code event listeners.
     */
    public registerListeners(context: vscode.ExtensionContext): void {
        // File opened (immediate, no debounce)
        this.disposables.push(
            vscode.workspace.onDidOpenTextDocument((document) => {
                this.onFileOpened(document);
            })
        );

        // Cursor moved (debounced)
        this.disposables.push(
            vscode.window.onDidChangeTextEditorSelection((event) => {
                this.cursorDebouncer(event);
            })
        );

        // Text edited (debounced)
        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument((event) => {
                this.editDebouncer(event);
            })
        );

        context.subscriptions.push(...this.disposables);
    }

    /**
     * Handle file opened event (immediate).
     */
    private onFileOpened(document: vscode.TextDocument): void {
        if (!this.enabled) return;
        if (document.uri.scheme !== 'file') return;

        const event: IdeEvent = {
            event_type: 'file_opened',
            file_path: document.uri.fsPath,
        };

        this.enqueueEvent(event);
    }

    /**
     * Handle cursor moved event (debounced).
     */
    private async handleCursorMoved(
        event: vscode.TextEditorSelectionChangeEvent
    ): Promise<void> {
        if (!this.enabled) return;
        if (!event.textEditor.document) return;
        if (event.textEditor.document.uri.scheme !== 'file') return;

        const document = event.textEditor.document;
        const position = event.selections[0]?.active;
        if (!position) return;

        // Extract symbol at cursor
        const symbol = await this.symbolExtractor.getSymbolAtPosition(
            document,
            position
        );

        const ideEvent: IdeEvent = {
            event_type: 'cursor_moved',
            file_path: document.uri.fsPath,
            cursor_line: position.line + 1, // Convert to 1-based
            symbol: symbol,
        };

        this.enqueueEvent(ideEvent);
    }

    /**
     * Handle text edited event (debounced).
     */
    private handleTextEdited(event: vscode.TextDocumentChangeEvent): void {
        if (!this.enabled) return;
        if (event.document.uri.scheme !== 'file') return;

        const ideEvent: IdeEvent = {
            event_type: 'text_edited',
            file_path: event.document.uri.fsPath,
        };

        this.enqueueEvent(ideEvent);
    }

    /**
     * Add event to queue and send if possible.
     */
    private enqueueEvent(event: IdeEvent): void {
        this.eventQueue.push(event);

        // Discard oldest events if queue is full
        if (this.eventQueue.length > this.config.maxQueueSize) {
            this.eventQueue.shift();
            console.warn(
                `Event queue full (${this.config.maxQueueSize}), discarding oldest event`
            );
        }

        // Try to send queued events
        this.sendQueuedEvents();
    }

    /**
     * Send all queued events to daemon.
     */
    private async sendQueuedEvents(): Promise<void> {
        while (this.eventQueue.length > 0) {
            const event = this.eventQueue.shift()!;

            try {
                await this.sendIpcRequest('ide_event', event);
            } catch (err) {
                // IPC not connected or error - don't re-queue, just log
                console.debug('Failed to send IDE event:', err);
                break; // Stop trying if IPC is down
            }
        }
    }

    /**
     * Enable or disable event tracking.
     */
    public setEnabled(enabled: boolean): void {
        this.enabled = enabled;
        if (!enabled) {
            // Clear queue when disabled
            this.eventQueue = [];
            this.cursorDebouncer.cancel();
            this.editDebouncer.cancel();
        }
    }

    /**
     * Update debounce timing.
     */
    public setDebounceMs(debounceMs: number): void {
        this.config.debounceMs = debounceMs;
        // Recreate debouncers with new timing
        this.cursorDebouncer = debounce(
            (event) => this.handleCursorMoved(event),
            debounceMs
        );
        this.editDebouncer = debounce(
            (event) => this.handleTextEdited(event),
            debounceMs
        );
    }

    /**
     * Get current queue size (for testing).
     */
    public getQueueSize(): number {
        return this.eventQueue.length;
    }

    /**
     * Dispose all event listeners.
     */
    public dispose(): void {
        this.cursorDebouncer.cancel();
        this.editDebouncer.cancel();
        this.disposables.forEach((d) => d.dispose());
        this.disposables = [];
    }
}
