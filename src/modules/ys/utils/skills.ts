import type { YsCharacter, YsSkill } from '../types';

/**
 * 可升级技能：等级上限大于 1。
 * 等级上限为 1 的被动/天赋不参与展示、统计与筛选。
 */
export function upgradableSkills(character: YsCharacter): YsSkill[] {
    return character.skillList.filter((skill) => skill.maxLevel > 1);
}

/** 可升级技能的当前等级（按技能列表顺序） */
export function upgradableSkillLevels(character: YsCharacter): number[] {
    return upgradableSkills(character).map((skill) => skill.levelCurrent);
}
