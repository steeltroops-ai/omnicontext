/**
 * Cache statistics management.
 * Retrieves and formats cache statistics from the daemon.
 */

import { CacheStats } from './types';

export class CacheStatsManager {
    constructor(
        private sendIpcRequest: (method: string, params: any) => Promise<any>
    ) { }

    /**
     * Retrieve cache statistics from daemon.
     * Returns null if daemon is not connected or request fails.
     */
    public async getStats(): Promise<CacheStats | null> {
        try {
            const stats = await this.sendIpcRequest('prefetch_stats', {});
            return {
                hits: stats.hits || 0,
                misses: stats.misses || 0,
                size: stats.size || 0,
                capacity: 100, // Default capacity
                hit_rate: stats.hit_rate || 0,
            };
        } catch (err) {
            console.debug('Failed to get cache stats:', err);
            return null;
        }
    }

    /**
     * Clear the pre-fetch cache.
     */
    public async clearCache(): Promise<void> {
        try {
            await this.sendIpcRequest('clear_cache', {});
        } catch (err) {
            console.error('Failed to clear cache:', err);
            throw new Error(`Failed to clear cache: ${err}`);
        }
    }

    /**
     * Format cache statistics for display.
     */
    public formatStats(stats: CacheStats): string {
        const hitRate = (stats.hit_rate * 100).toFixed(1);
        return `Hit Rate: ${hitRate}% | Hits: ${stats.hits} | Misses: ${stats.misses} | Size: ${stats.size}/${stats.capacity}`;
    }

    /**
     * Get cache status indicator.
     */
    public getStatusIndicator(
        stats: CacheStats | null,
        daemonConnected: boolean,
        prefetchEnabled: boolean
    ): { text: string; icon: string; cssClass: string } {
        if (!daemonConnected) {
            return {
                text: 'Offline',
                icon: 'warning',
                cssClass: 'offline',
            };
        }

        if (!prefetchEnabled) {
            return {
                text: 'Disabled',
                icon: 'circle-slash',
                cssClass: 'disabled',
            };
        }

        if (stats && stats.hit_rate > 0) {
            return {
                text: 'Active',
                icon: 'zap',
                cssClass: 'active',
            };
        }

        return {
            text: 'Warming Up',
            icon: 'sync',
            cssClass: 'warming',
        };
    }
}
