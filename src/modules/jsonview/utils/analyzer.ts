import type { JsonAnalysis, JsonNode, JsonStats, JsonValueType } from '../types';

/** 单个节点最多展开的子节点数，避免大数组撑爆界面 */
export const MAX_CHILDREN_PER_NODE = 100;
/** 整棵树最多构建的节点数 */
export const MAX_NODES = 5000;
/** 最大递归层级，超过后不再展开 */
export const MAX_DEPTH = 50;

const TYPE_LABELS: Record<JsonValueType, string> = {
    object: '对象',
    array: '数组',
    string: '字符串',
    number: '数字',
    boolean: '布尔',
    null: 'null',
};

export function jsonTypeLabel(type: JsonValueType): string {
    return TYPE_LABELS[type];
}

function kindOf(value: unknown): JsonValueType {
    if (value === null) return 'null';
    if (Array.isArray(value)) return 'array';
    switch (typeof value) {
        case 'object':
            return 'object';
        case 'string':
            return 'string';
        case 'number':
            return 'number';
        case 'boolean':
            return 'boolean';
        default:
            return 'null';
    }
}

/** 生成对象键对应的路径片段 */
function childPath(parent: string, key: string): string {
    if (/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(key)) return `${parent}.${key}`;
    return `${parent}[${JSON.stringify(key)}]`;
}

/** 原始值预览，长字符串截断 */
function previewValue(value: unknown): string {
    if (typeof value === 'string') {
        const quoted = JSON.stringify(value);
        return quoted.length > 80 ? `${quoted.slice(0, 77)}...` : quoted;
    }
    if (value === null) return 'null';
    return String(value);
}

interface Counter {
    nodes: number;
    objects: number;
    arrays: number;
    primitives: number;
    maxDepth: number;
    exhausted: boolean;
}

function buildNode(
    value: unknown,
    key: string,
    path: string,
    depth: number,
    counter: Counter,
): JsonNode {
    const type = kindOf(value);
    counter.nodes += 1;
    counter.maxDepth = Math.max(counter.maxDepth, depth);
    if (type === 'object') counter.objects += 1;
    else if (type === 'array') counter.arrays += 1;
    else counter.primitives += 1;

    let childCount = 0;
    if (type === 'object') {
        childCount = Object.keys(value as Record<string, unknown>).length;
    } else if (type === 'array') {
        childCount = (value as unknown[]).length;
    }

    const node: JsonNode = {
        key,
        path,
        type,
        value: type === 'object' ? `{${childCount}}` : type === 'array' ? `[${childCount}]` : previewValue(value),
        children: [],
        childCount,
        truncated: false,
        depth,
    };

    if (type !== 'object' && type !== 'array') return node;

    if (depth >= MAX_DEPTH) {
        node.truncated = childCount > 0;
        return node;
    }

    if (counter.nodes >= MAX_NODES) {
        counter.exhausted = true;
        node.truncated = childCount > 0;
        return node;
    }

    if (type === 'object') {
        const entries = Object.entries(value as Record<string, unknown>);
        for (const [childKey, childValue] of entries) {
            if (counter.exhausted || node.children.length >= MAX_CHILDREN_PER_NODE) {
                node.truncated = true;
                break;
            }
            node.children.push(
                buildNode(childValue, childKey, childPath(path, childKey), depth + 1, counter),
            );
        }
    } else {
        const items = value as unknown[];
        for (let i = 0; i < items.length; i += 1) {
            if (counter.exhausted || node.children.length >= MAX_CHILDREN_PER_NODE) {
                node.truncated = true;
                break;
            }
            node.children.push(buildNode(items[i], `[${i}]`, `${path}[${i}]`, depth + 1, counter));
        }
    }

    return node;
}

/** 从 JSON.parse 的错误信息中提取位置，并换算为行列号 */
function describeParseError(text: string, error: unknown): string {
    const message = error instanceof Error ? error.message : String(error);

    const lineColumn = /line\s+(\d+)\s+column\s+(\d+)/i.exec(message);
    if (lineColumn) {
        return `JSON 解析失败（第 ${lineColumn[1]} 行，第 ${lineColumn[2]} 列）：${message}`;
    }

    const positionMatch = /position\s+(\d+)/i.exec(message);
    if (positionMatch) {
        const position = Math.min(Number(positionMatch[1]), text.length);
        let line = 1;
        let column = 1;
        for (let i = 0; i < position; i += 1) {
            if (text[i] === '\n') {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        return `JSON 解析失败（第 ${line} 行，第 ${column} 列）：${message}`;
    }

    return `JSON 解析失败：${message}`;
}

/**
 * 分析 JSON 文本，返回根节点与统计信息。
 * 解析失败时抛出带中文前缀的错误；节点数量/深度超限时截断并设置 truncated。
 */
export function analyzeJsonText(text: string): JsonAnalysis {
    let parsed: unknown;
    try {
        parsed = JSON.parse(text);
    } catch (error) {
        throw new Error(describeParseError(text, error));
    }

    const counter: Counter = {
        nodes: 0,
        objects: 0,
        arrays: 0,
        primitives: 0,
        maxDepth: 0,
        exhausted: false,
    };

    const root = buildNode(parsed, '$', '$', 0, counter);
    const stats: JsonStats = {
        rootType: root.type,
        totalNodes: counter.nodes,
        maxDepth: counter.maxDepth,
        objects: counter.objects,
        arrays: counter.arrays,
        primitives: counter.primitives,
        truncated: counter.exhausted,
    };

    return { root, stats };
}
