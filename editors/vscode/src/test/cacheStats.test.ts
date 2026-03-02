/**
 * Unit tests for CacheStatsManager.
 */

import * as assert from 'assert';
import { CacheStatsManager } from '../cacheStats';
import { CacheStats } from '../types';

suite('CacheStatsManager', () => {
    suite('getStats', () => {
        test('should retrieve cache statistics successfully', async () => {
            const mockIpcRequest = async (method: string, params: any) => {
                assert.strictEqual(method, 'prefetch_stats');
                return {
                    hits: 50,
                    misses: 25,
                    size: 45,
                    hit_rate: 0.667,
                };
            };

            const manager = new CacheStatsManager(mockIpcRequest);
            const stats = await manager.getStats();

            assert.ok(stats);
            assert.strictEqual(stats.hits, 50);
            assert.strictEqual(stats.misses, 25);
            assert.strictEqual(stats.size, 45);
            assert.strictEqual(stats.capacity, 100);
            assert.strictEqual(stats.hit_rate, 0.667);
        });

        test('should handle missing fields with defaults', async () => {
            const mockIpcRequest = async (method: string, params: any) => {
                return {}; // Empty response
            };

            const manager = new CacheStatsManager(mockIpcRequest);
            const stats = await manager.getStats();

            assert.ok(stats);
            assert.strictEqual(stats.hits, 0);
            assert.strictEqual(stats.misses, 0);
            assert.strictEqual(stats.size, 0);
            assert.strictEqual(stats.capacity, 100);
            assert.strictEqual(stats.hit_rate, 0);
        });

        test('should return null when daemon is offline', async () => {
            const mockIpcRequest = async (method: string, params: any) => {
                throw new Error('Daemon not connected');
            };

            const manager = new CacheStatsManager(mockIpcRequest);
            const stats = await manager.getStats();

            assert.strictEqual(stats, null);
        });

        test('should return null on IPC failure', async () => {
            const mockIpcRequest = async (method: string, params: any) => {
                throw new Error('IPC timeout');
            };

            const manager = new CacheStatsManager(mockIpcRequest);
            const stats = await manager.getStats();

            assert.strictEqual(stats, null);
        });
    });

    suite('clearCache', () => {
        test('should clear cache successfully', async () => {
            let clearCalled = false;
            const mockIpcRequest = async (method: string, params: any) => {
                assert.strictEqual(method, 'clear_cache');
                clearCalled = true;
                return { cleared: true };
            };

            const manager = new CacheStatsManager(mockIpcRequest);
            await manager.clearCache();

            assert.ok(clearCalled);
        });

        test('should throw error when clear fails', async () => {
            const mockIpcRequest = async (method: string, params: any) => {
                throw new Error('Daemon not connected');
            };

            const manager = new CacheStatsManager(mockIpcRequest);

            await assert.rejects(
                async () => await manager.clearCache(),
                /Failed to clear cache/
            );
        });

        test('should propagate IPC errors', async () => {
            const mockIpcRequest = async (method: string, params: any) => {
                throw new Error('Permission denied');
            };

            const manager = new CacheStatsManager(mockIpcRequest);

            await assert.rejects(
                async () => await manager.clearCache(),
                /Permission denied/
            );
        });
    });

    suite('formatStats', () => {
        test('should format stats with high hit rate', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 234,
                misses: 115,
                size: 45,
                capacity: 100,
                hit_rate: 0.67,
            };

            const formatted = manager.formatStats(stats);

            assert.strictEqual(
                formatted,
                'Hit Rate: 67.0% | Hits: 234 | Misses: 115 | Size: 45/100'
            );
        });

        test('should format stats with zero hit rate', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 0,
                misses: 10,
                size: 5,
                capacity: 100,
                hit_rate: 0,
            };

            const formatted = manager.formatStats(stats);

            assert.strictEqual(
                formatted,
                'Hit Rate: 0.0% | Hits: 0 | Misses: 10 | Size: 5/100'
            );
        });

        test('should format stats with 100% hit rate', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 100,
                misses: 0,
                size: 50,
                capacity: 100,
                hit_rate: 1.0,
            };

            const formatted = manager.formatStats(stats);

            assert.strictEqual(
                formatted,
                'Hit Rate: 100.0% | Hits: 100 | Misses: 0 | Size: 50/100'
            );
        });

        test('should format stats with decimal hit rate', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 123,
                misses: 456,
                size: 78,
                capacity: 100,
                hit_rate: 0.21244,
            };

            const formatted = manager.formatStats(stats);

            assert.strictEqual(
                formatted,
                'Hit Rate: 21.2% | Hits: 123 | Misses: 456 | Size: 78/100'
            );
        });

        test('should format stats with full cache', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 500,
                misses: 200,
                size: 100,
                capacity: 100,
                hit_rate: 0.714,
            };

            const formatted = manager.formatStats(stats);

            assert.strictEqual(
                formatted,
                'Hit Rate: 71.4% | Hits: 500 | Misses: 200 | Size: 100/100'
            );
        });
    });

    suite('getStatusIndicator', () => {
        test('should return offline status when daemon not connected', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 50,
                misses: 25,
                size: 45,
                capacity: 100,
                hit_rate: 0.67,
            };

            const status = manager.getStatusIndicator(stats, false, true);

            assert.strictEqual(status.text, 'Offline');
            assert.strictEqual(status.icon, 'warning');
            assert.strictEqual(status.cssClass, 'offline');
        });

        test('should return disabled status when prefetch disabled', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 50,
                misses: 25,
                size: 45,
                capacity: 100,
                hit_rate: 0.67,
            };

            const status = manager.getStatusIndicator(stats, true, false);

            assert.strictEqual(status.text, 'Disabled');
            assert.strictEqual(status.icon, 'circle-slash');
            assert.strictEqual(status.cssClass, 'disabled');
        });

        test('should return active status with cache hits', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 50,
                misses: 25,
                size: 45,
                capacity: 100,
                hit_rate: 0.67,
            };

            const status = manager.getStatusIndicator(stats, true, true);

            assert.strictEqual(status.text, 'Active');
            assert.strictEqual(status.icon, 'zap');
            assert.strictEqual(status.cssClass, 'active');
        });

        test('should return warming up status with no hits', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 0,
                misses: 5,
                size: 3,
                capacity: 100,
                hit_rate: 0,
            };

            const status = manager.getStatusIndicator(stats, true, true);

            assert.strictEqual(status.text, 'Warming Up');
            assert.strictEqual(status.icon, 'sync');
            assert.strictEqual(status.cssClass, 'warming');
        });

        test('should return warming up status when stats is null', () => {
            const manager = new CacheStatsManager(async () => ({}));

            const status = manager.getStatusIndicator(null, true, true);

            assert.strictEqual(status.text, 'Warming Up');
            assert.strictEqual(status.icon, 'sync');
            assert.strictEqual(status.cssClass, 'warming');
        });

        test('should prioritize offline over disabled', () => {
            const manager = new CacheStatsManager(async () => ({}));
            const stats: CacheStats = {
                hits: 50,
                misses: 25,
                size: 45,
                capacity: 100,
                hit_rate: 0.67,
            };

            const status = manager.getStatusIndicator(stats, false, false);

            assert.strictEqual(status.text, 'Offline');
            assert.strictEqual(status.icon, 'warning');
            assert.strictEqual(status.cssClass, 'offline');
        });
    });
});
