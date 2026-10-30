/** 筛选项的字符串值 + 中文标签 */
export interface LabelOption {
    value: string;
    label: string;
}

/** 元素类型（设定.txt：1 火；2 风；3 岩；4 草；5 雷；6 水；7 冰） */
export const ELEMENT_OPTIONS: LabelOption[] = [
    { value: '1', label: '火' },
    { value: '2', label: '风' },
    { value: '3', label: '岩' },
    { value: '4', label: '草' },
    { value: '5', label: '雷' },
    { value: '6', label: '水' },
    { value: '7', label: '冰' },
];

/** 武器类型（设定.txt：1 单手剑；10 法器；11 双手剑；12 弓箭；13 长枪） */
export const WEAPON_OPTIONS: LabelOption[] = [
    { value: '1', label: '单手剑' },
    { value: '10', label: '法器' },
    { value: '11', label: '双手剑' },
    { value: '12', label: '弓箭' },
    { value: '13', label: '长枪' },
];

/** 圣遗物类型（设定.txt：1 生之花；2 死之羽；3 时之沙；4 空之杯；5 理之冠） */
export const RELIQUARY_OPTIONS: LabelOption[] = [
    { value: '1', label: '生之花' },
    { value: '2', label: '死之羽' },
    { value: '3', label: '时之沙' },
    { value: '4', label: '空之杯' },
    { value: '5', label: '理之冠' },
];

/** 角色星级 */
export const RARITY_OPTIONS: LabelOption[] = [
    { value: '5', label: '5 星' },
    { value: '4', label: '4 星' },
];

function labelOf(options: LabelOption[], id: number, fallback: string): string {
    return options.find((item) => item.value === String(id))?.label ?? fallback;
}

export function elementLabel(id: number): string {
    return labelOf(ELEMENT_OPTIONS, id, '未知');
}

export function weaponCatLabel(id: number): string {
    return labelOf(WEAPON_OPTIONS, id, '未知');
}

export function reliquaryCatLabel(id: number): string {
    return labelOf(RELIQUARY_OPTIONS, id, '未知');
}
