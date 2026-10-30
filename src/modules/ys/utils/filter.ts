import type { YsCharacter } from '../types';
import {
    ELEMENT_OPTIONS,
    RARITY_OPTIONS,
    RELIQUARY_OPTIONS,
    WEAPON_OPTIONS,
    type LabelOption,
} from './labels';
import { upgradableSkillLevels, upgradableSkills } from './skills';

export type FilterFieldKey =
    | 'name'
    | 'id'
    | 'elementAttrId'
    | 'weaponCatId'
    | 'avatarLevel'
    | 'levelCurrent'
    | 'maxLevel'
    | 'constellationNum'
    | 'fetterLevel'
    | 'weaponName'
    | 'weaponLevel'
    | 'weaponCurrentLevel'
    | 'skillName'
    | 'skillLevel'
    | 'skillCount'
    | 'reliquaryName'
    | 'reliquaryCount'
    | 'reliquaryCatId';

export type FilterFieldKind = 'text' | 'number' | 'enum' | 'enumList' | 'numberList';

export type FilterOperator =
    | 'eq'
    | 'ne'
    | 'contains'
    | 'not_contains'
    | 'gt'
    | 'gte'
    | 'lt'
    | 'lte';

export interface FilterField {
    key: FilterFieldKey;
    label: string;
    kind: FilterFieldKind;
    options?: LabelOption[];
    get(character: YsCharacter): string | number | number[];
}

export interface FilterCondition {
    id: number;
    field: FilterFieldKey;
    operator: FilterOperator;
    value: string;
}

export type FilterConjunction = 'and' | 'or';

/** 顶部快捷筛选条件 */
export interface QuickFilter {
    keyword: string;
    elementAttrId: string;
    weaponCatId: string;
    avatarLevel: string;
    levelMin: string;
    levelMax: string;
}

export const FILTER_FIELDS: FilterField[] = [
    { key: 'name', label: '角色名', kind: 'text', get: (c) => c.name },
    { key: 'id', label: '角色 ID', kind: 'number', get: (c) => c.id },
    {
        key: 'elementAttrId',
        label: '元素',
        kind: 'enum',
        options: ELEMENT_OPTIONS,
        get: (c) => c.elementAttrId,
    },
    {
        key: 'weaponCatId',
        label: '武器类型',
        kind: 'enum',
        options: WEAPON_OPTIONS,
        get: (c) => c.weaponCatId,
    },
    {
        key: 'avatarLevel',
        label: '角色星级',
        kind: 'enum',
        options: RARITY_OPTIONS,
        get: (c) => c.avatarLevel,
    },
    { key: 'levelCurrent', label: '当前等级', kind: 'number', get: (c) => c.levelCurrent },
    { key: 'maxLevel', label: '最大等级', kind: 'number', get: (c) => c.maxLevel },
    { key: 'constellationNum', label: '命座等级', kind: 'number', get: (c) => c.constellationNum },
    { key: 'fetterLevel', label: '好感等级', kind: 'number', get: (c) => c.fetterLevel },
    { key: 'weaponName', label: '武器名称', kind: 'text', get: (c) => c.weapon?.name ?? '' },
    {
        key: 'weaponLevel',
        label: '武器星级',
        kind: 'number',
        get: (c) => c.weapon?.weaponLevel ?? 0,
    },
    {
        key: 'weaponCurrentLevel',
        label: '武器等级',
        kind: 'number',
        get: (c) => c.weapon?.levelCurrent ?? 0,
    },
    {
        key: 'skillName',
        label: '技能名称',
        kind: 'text',
        get: (c) => upgradableSkills(c).map((skill) => skill.name).join(' '),
    },
    {
        key: 'skillLevel',
        label: '技能等级（任一）',
        kind: 'numberList',
        get: (c) => upgradableSkillLevels(c),
    },
    {
        key: 'skillCount',
        label: '技能数量（可升级）',
        kind: 'number',
        get: (c) => upgradableSkills(c).length,
    },
    {
        key: 'reliquaryName',
        label: '圣遗物名称',
        kind: 'text',
        get: (c) => c.reliquaryList.map((item) => item.name).join(' '),
    },
    {
        key: 'reliquaryCount',
        label: '圣遗物数量',
        kind: 'number',
        get: (c) => c.reliquaryList.length,
    },
    {
        key: 'reliquaryCatId',
        label: '圣遗物类型',
        kind: 'enumList',
        options: RELIQUARY_OPTIONS,
        get: (c) => c.reliquaryList.map((item) => item.reliquaryCatId),
    },
];

export const OPERATOR_LABELS: Record<FilterOperator, string> = {
    eq: '等于',
    ne: '不等于',
    contains: '包含',
    not_contains: '不包含',
    gt: '大于',
    gte: '大于等于',
    lt: '小于',
    lte: '小于等于',
};

const OPERATORS_BY_KIND: Record<FilterFieldKind, FilterOperator[]> = {
    text: ['contains', 'eq', 'ne', 'not_contains'],
    number: ['eq', 'ne', 'gt', 'gte', 'lt', 'lte'],
    enum: ['eq', 'ne'],
    enumList: ['contains', 'not_contains'],
    numberList: ['eq', 'ne', 'gt', 'gte', 'lt', 'lte'],
};

const FIELD_MAP = new Map(FILTER_FIELDS.map((field) => [field.key, field]));

export function fieldOf(key: FilterFieldKey): FilterField {
    return FIELD_MAP.get(key) ?? FILTER_FIELDS[0]!;
}

export function operatorsFor(kind: FilterFieldKind): FilterOperator[] {
    return OPERATORS_BY_KIND[kind];
}

/** 数字输入框的 v-model 可能得到 number，统一按字符串处理 */
function trimmed(value: unknown): string {
    if (typeof value === 'string') return value.trim();
    if (value === null || value === undefined) return '';
    return String(value).trim();
}

export function isConditionActive(condition: FilterCondition): boolean {
    return trimmed(condition.value) !== '';
}

function compareNumber(value: number, operator: FilterOperator, target: number): boolean {
    switch (operator) {
        case 'eq':
            return value === target;
        case 'ne':
            return value !== target;
        case 'gt':
            return value > target;
        case 'gte':
            return value >= target;
        case 'lt':
            return value < target;
        case 'lte':
            return value <= target;
        default:
            return true;
    }
}

function matchCondition(character: YsCharacter, condition: FilterCondition): boolean {
    if (!isConditionActive(condition)) return true;

    const field = fieldOf(condition.field);
    const raw = field.get(character);
    const input = trimmed(condition.value);

    if (field.kind === 'text') {
        const text = String(raw).toLocaleLowerCase();
        const target = input.toLocaleLowerCase();
        switch (condition.operator) {
            case 'eq':
                return text === target;
            case 'ne':
                return text !== target;
            case 'contains':
                return text.includes(target);
            case 'not_contains':
                return !text.includes(target);
            default:
                return true;
        }
    }

    if (field.kind === 'number') {
        const target = Number(input);
        if (!Number.isFinite(target)) return true;
        return compareNumber(Number(raw), condition.operator, target);
    }

    if (field.kind === 'enum') {
        const target = Number(input);
        if (!Number.isFinite(target)) return true;
        const value = Number(raw);
        return condition.operator === 'ne' ? value !== target : value === target;
    }

    if (field.kind === 'numberList') {
        const target = Number(input);
        if (!Number.isFinite(target)) return true;
        const values = Array.isArray(raw) ? raw.map(Number) : [];
        // 「不等于」按所有技能都不等于目标值处理，其余运算符按任一技能满足处理
        if (condition.operator === 'ne') {
            return values.every((value) => value !== target);
        }
        return values.some((value) => compareNumber(value, condition.operator, target));
    }

    const target = Number(input);
    if (!Number.isFinite(target)) return true;
    const values = Array.isArray(raw) ? raw : [];
    const hasValue = values.includes(target);
    return condition.operator === 'not_contains' ? !hasValue : hasValue;
}

function matchesQuickFilter(character: YsCharacter, filter: QuickFilter): boolean {
    const keyword = trimmed(filter.keyword).toLocaleLowerCase();
    if (keyword) {
        const haystack = [
            character.name,
            character.weapon?.name ?? '',
            ...upgradableSkills(character).map((skill) => skill.name),
            ...character.reliquaryList.map((item) => item.name),
        ]
            .join(' ')
            .toLocaleLowerCase();
        if (!haystack.includes(keyword)) return false;
    }

    if (filter.elementAttrId && String(character.elementAttrId) !== filter.elementAttrId) {
        return false;
    }
    if (filter.weaponCatId && String(character.weaponCatId) !== filter.weaponCatId) {
        return false;
    }
    if (filter.avatarLevel && String(character.avatarLevel) !== filter.avatarLevel) {
        return false;
    }

    const levelMinText = trimmed(filter.levelMin);
    const levelMin = Number(levelMinText);
    if (levelMinText && Number.isFinite(levelMin) && character.levelCurrent < levelMin) {
        return false;
    }
    const levelMaxText = trimmed(filter.levelMax);
    const levelMax = Number(levelMaxText);
    if (levelMaxText && Number.isFinite(levelMax) && character.levelCurrent > levelMax) {
        return false;
    }

    return true;
}

/** 固定筛选条件（不占用自定义条件，始终与其余条件同时满足） */
export interface FixedFilter {
    /** 可升级技能中任意一个低于该等级时命中；null 表示不启用 */
    anySkillBelow: number | null;
}

export const EMPTY_FIXED_FILTER: FixedFilter = { anySkillBelow: null };

/** 快捷筛选、固定条件与自定义条件同时生效；自定义条件之间按 conjunction 组合 */
export function filterCharacters(
    characters: YsCharacter[],
    quick: QuickFilter,
    conditions: FilterCondition[],
    conjunction: FilterConjunction,
    fixed: FixedFilter = EMPTY_FIXED_FILTER,
): YsCharacter[] {
    const activeConditions = conditions.filter(isConditionActive);
    const skillThreshold = fixed.anySkillBelow;

    return characters.filter((character) => {
        if (
            skillThreshold !== null &&
            !upgradableSkillLevels(character).some((level) => level < skillThreshold)
        ) {
            return false;
        }
        if (!matchesQuickFilter(character, quick)) return false;
        if (activeConditions.length === 0) return true;
        return conjunction === 'or'
            ? activeConditions.some((condition) => matchCondition(character, condition))
            : activeConditions.every((condition) => matchCondition(character, condition));
    });
}
