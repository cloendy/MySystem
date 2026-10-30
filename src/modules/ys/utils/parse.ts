import type {
    YsCharacter,
    YsDataset,
    YsReliquary,
    YsSkill,
    YsWeapon,
} from '../types';

function asRecord(value: unknown): Record<string, unknown> | null {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) return null;
    return value as Record<string, unknown>;
}

function asNumber(value: unknown, fallback = 0): number {
    return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function asString(value: unknown, fallback = ''): string {
    return typeof value === 'string' ? value : fallback;
}

function asArray(value: unknown): unknown[] {
    return Array.isArray(value) ? value : [];
}

function mapSkill(value: unknown): YsSkill | null {
    const item = asRecord(value);
    if (!item) return null;
    return {
        id: asNumber(item.id),
        groupId: asNumber(item.group_id),
        name: asString(item.name),
        icon: asString(item.icon),
        maxLevel: asNumber(item.max_level),
        levelCurrent: asNumber(item.level_current),
    };
}

/** weapon 在设定中写的是数组，实际数据是对象，两种都兼容 */
function mapWeapon(value: unknown): YsWeapon | null {
    const candidates = Array.isArray(value) ? value : [value];
    for (const candidate of candidates) {
        const item = asRecord(candidate);
        if (!item) continue;
        return {
            id: asNumber(item.id),
            name: asString(item.name),
            icon: asString(item.icon),
            weaponCatId: asNumber(item.weapon_cat_id),
            weaponLevel: asNumber(item.weapon_level),
            maxLevel: asNumber(item.max_level),
            levelCurrent: asNumber(item.level_current),
        };
    }
    return null;
}

function mapReliquary(value: unknown): YsReliquary | null {
    const item = asRecord(value);
    if (!item) return null;
    return {
        id: asNumber(item.id),
        name: asString(item.name),
        icon: asString(item.icon),
        reliquaryCatId: asNumber(item.reliquary_cat_id),
        reliquaryLevel: asNumber(item.reliquary_level),
        levelCurrent: asNumber(item.level_current),
        maxLevel: asNumber(item.max_level),
    };
}

function mapCharacter(value: unknown): YsCharacter | null {
    const item = asRecord(value);
    if (!item) return null;

    const name = asString(item.name);
    if (!name) return null;

    return {
        id: asNumber(item.id),
        name,
        icon: asString(item.icon),
        weaponCatId: asNumber(item.weapon_cat_id),
        avatarLevel: asNumber(item.avatar_level),
        elementAttrId: asNumber(item.element_attr_id),
        maxLevel: asNumber(item.max_level),
        levelCurrent: asNumber(item.level_current),
        promoteLevel: asNumber(item.promote_level),
        skillList: asArray(item.skill_list)
            .map(mapSkill)
            .filter((skill): skill is YsSkill => skill !== null),
        weapon: mapWeapon(item.weapon),
        reliquaryList: asArray(item.reliquary_list)
            .map(mapReliquary)
            .filter((item): item is YsReliquary => item !== null),
        wikiUrl: asString(item.wiki_url),
        wikiRecommendWeaponUrl: asString(item.wiki_recommend_weapon_url),
        constellationNum: asNumber(item.constellation_num),
        fetterLevel: asNumber(item.fetter_level),
    };
}

/** 兼容 `{data:{list}}`、`{list}` 与顶层数组三种结构 */
function extractList(root: unknown): unknown[] {
    if (Array.isArray(root)) return root;

    const rootRecord = asRecord(root);
    if (rootRecord) {
        const data = asRecord(rootRecord.data);
        if (data && Array.isArray(data.list)) return data.list;
        if (Array.isArray(rootRecord.list)) return rootRecord.list;
    }

    throw new Error('未找到角色列表（data.list）');
}

/** 解析 ys.json 文本，返回归一化后的角色列表 */
export function parseYsCharacters(text: string): YsDataset {
    let parsed: unknown;
    try {
        parsed = JSON.parse(text);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(`ys.json 解析失败：${message}`);
    }

    const rawList = extractList(parsed);
    const characters = rawList
        .map(mapCharacter)
        .filter((character): character is YsCharacter => character !== null);

    if (rawList.length > 0 && characters.length === 0) {
        throw new Error('ys.json 中没有可识别的角色数据');
    }

    return { characters, total: characters.length };
}
