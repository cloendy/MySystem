import type { Component } from 'vue';

export type Platform = 'windows' | 'android';

export interface ModuleRoute {
    path: string;
    name: string;
    component: () => Promise<Component>;
    meta?: Record<string, unknown>;
}

export interface SearchResult {
    moduleId: string;
    documentId: string;
    title: string;
    snippet: string;
}

/**
 * 模块接口：所有功能模块必须实现
 */
export interface ModuleInterface {
    id: string;
    name: string;
    version: string;

    onLoad(context: ModuleContext): Promise<void>;
    onUnload?(): Promise<void>;
    onActivate?(): void;
    onDeactivate?(): void;

    getRoutes(): ModuleRoute[];
    onSearch?(query: string): Promise<SearchResult[]>;
}

/**
 * 模块上下文：平台注入给模块的服务
 */
export interface ModuleContext {
    platform: Platform;
    events: EventBus;
    // 后续会加入 storage、sync、config 等
}

/**
 * 简化版事件总线
 */
export interface EventBus {
    on(event: string, handler: (payload: unknown) => void): void;
    off(event: string, handler: (payload: unknown) => void): void;
    emit(event: string, payload?: unknown): void;
}

/**
 * 模块注册表中的条目（对应 modules.json 中一项）
 */
export interface ModuleRegistration {
    id: string;
    name: string;
    version: string;
    entryPoint: string;
    enabled: Record<Platform, boolean>;
}