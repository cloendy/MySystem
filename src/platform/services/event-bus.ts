import { logger } from './logger'
import type { EventBus } from './module-types';

export function createEventBus(): EventBus {
    const handlers = new Map<string, Set<(payload: unknown) => void>>();

    return {
        on(event, handler) {
            if (!handlers.has(event)) {
                handlers.set(event, new Set());
            }
            handlers.get(event)!.add(handler);
        },
        off(event, handler) {
            handlers.get(event)?.delete(handler);
        },
        emit(event, payload) {
            handlers.get(event)?.forEach((h) => {
                try {
                    h(payload);
                } catch (err) {
                    logger.error(`[EventBus] handler error for ${event}:`, err);
                }
            });
        },
    };
}