/** JSON 值的运行时类型 */
export type JsonValueType = 'object' | 'array' | 'string' | 'number' | 'boolean' | 'null';

/** 分析结果中的一个节点，对应 JSON 树中的一个值 */
export interface JsonNode {
    /** 对象键名；数组元素为 `[索引]`；根节点为 `$` */
    key: string;
    /** 类 JSONPath 路径，如 `$.data[0].name` */
    path: string;
    type: JsonValueType;
    /** 原始值预览；对象/数组为 `{N}` / `[N]` */
    value: string;
    /** 子节点；对象为各字段，数组为各元素（受数量上限约束） */
    children: JsonNode[];
    /** 对象字段数或数组长度 */
    childCount: number;
    /** 当前节点是否有子节点因上限被截断 */
    truncated: boolean;
    /** 根节点为 0 */
    depth: number;
}

export interface JsonStats {
    rootType: JsonValueType;
    /** 实际构建的节点数（含根） */
    totalNodes: number;
    maxDepth: number;
    objects: number;
    arrays: number;
    primitives: number;
    /** 是否因节点总数上限提前结束分析 */
    truncated: boolean;
}

export interface JsonAnalysis {
    root: JsonNode;
    stats: JsonStats;
}

/** Rust `read_json_file` 的返回结构 */
export interface JsonFileContent {
    path: string;
    content: string;
    /** 文件字节数 */
    size: number;
}
