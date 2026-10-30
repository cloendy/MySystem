/** 角色技能（对应 ys.json 的 skill_list 项） */
export interface YsSkill {
    id: number;
    groupId: number;
    name: string;
    icon: string;
    maxLevel: number;
    levelCurrent: number;
}

/** 角色装备的武器（ys.json 中 weapon 为对象） */
export interface YsWeapon {
    id: number;
    name: string;
    icon: string;
    weaponCatId: number;
    /** 武器星级 1~5 */
    weaponLevel: number;
    maxLevel: number;
    levelCurrent: number;
}

/** 角色装备的圣遗物 */
export interface YsReliquary {
    id: number;
    name: string;
    icon: string;
    /** 1 生之花；2 死之羽；3 时之沙；4 空之杯；5 理之冠 */
    reliquaryCatId: number;
    /** 圣遗物星级 1~5 */
    reliquaryLevel: number;
    levelCurrent: number;
    maxLevel: number;
}

/** 一个角色的完整数据（字段已从 snake_case 归一为 camelCase） */
export interface YsCharacter {
    id: number;
    name: string;
    icon: string;
    /** 1 单手剑；10 法器；11 双手剑；12 弓箭；13 长枪 */
    weaponCatId: number;
    /** 角色星级 4/5 */
    avatarLevel: number;
    /** 1 火；2 风；3 岩；4 草；5 雷；6 水；7 冰 */
    elementAttrId: number;
    maxLevel: number;
    levelCurrent: number;
    promoteLevel: number;
    skillList: YsSkill[];
    weapon: YsWeapon | null;
    reliquaryList: YsReliquary[];
    wikiUrl: string;
    wikiRecommendWeaponUrl: string;
    constellationNum: number;
    fetterLevel: number;
}

/** 解析后的角色数据集合 */
export interface YsDataset {
    characters: YsCharacter[];
    total: number;
}

/** Rust `read_ys_file` 的返回结构；文件不存在时命令返回 null */
export interface YsFileContent {
    path: string;
    content: string;
    size: number;
}
