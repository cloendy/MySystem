<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { logger } from '../../../platform/services/logger';
import * as api from '../api';
import type { YsCharacter } from '../types';
import { parseYsCharacters } from '../utils/parse';
import { upgradableSkills } from '../utils/skills';
import {
    ELEMENT_OPTIONS,
    RARITY_OPTIONS,
    WEAPON_OPTIONS,
    elementLabel,
    reliquaryCatLabel,
    weaponCatLabel,
} from '../utils/labels';
import {
    FILTER_FIELDS,
    OPERATOR_LABELS,
    fieldOf,
    filterCharacters,
    isConditionActive,
    operatorsFor,
    type FilterCondition,
    type FilterConjunction,
    type QuickFilter,
} from '../utils/filter';

const loading = ref(true);
const missing = ref(false);
const error = ref('');
const filePath = ref('');
const fileSize = ref(0);
const characters = ref<YsCharacter[]>([]);

const keyword = ref('');
const elementFilter = ref('');
const weaponFilter = ref('');
const rarityFilter = ref('');
const levelMin = ref('');
const levelMax = ref('');
/** 固定筛选：三个可升级技能中任意一个低于 9 级 */
const fixedSkillBelow9 = ref(false);

const conjunction = ref<FilterConjunction>('and');
const conditions = ref<FilterCondition[]>([]);
let conditionSeq = 0;

interface AppliedFilter {
    quick: QuickFilter;
    conditions: FilterCondition[];
    conjunction: FilterConjunction;
    fixedSkillBelow9: boolean;
}

/** 只有点击「筛选」后才写入；为 null 时显示全部角色 */
const appliedFilter = ref<AppliedFilter | null>(null);
const appliedSignature = ref('');
const filterError = ref('');

const quickFilter = computed<QuickFilter>(() => ({
    keyword: keyword.value,
    elementAttrId: elementFilter.value,
    weaponCatId: weaponFilter.value,
    avatarLevel: rarityFilter.value,
    levelMin: levelMin.value,
    levelMax: levelMax.value,
}));

const activeConditions = computed(() => conditions.value.filter(isConditionActive));
const activeConditionCount = computed(() => activeConditions.value.length);
const incompleteConditionCount = computed(
    () => conditions.value.length - activeConditions.value.length,
);

const filteredCharacters = computed(() => {
    const applied = appliedFilter.value;
    if (!applied) return characters.value;
    return filterCharacters(
        characters.value,
        applied.quick,
        applied.conditions,
        applied.conjunction,
        { anySkillBelow: applied.fixedSkillBelow9 ? 9 : null },
    );
});

const appliedConditionCount = computed(
    () => appliedFilter.value?.conditions.length ?? 0,
);

/** 数字输入框的 v-model 可能得到 number，统一按字符串处理 */
function textOf(value: unknown): string {
    if (typeof value === 'string') return value.trim();
    if (value === null || value === undefined) return '';
    return String(value).trim();
}

function signatureOf(
    quick: QuickFilter,
    conditionList: FilterCondition[],
    relation: FilterConjunction,
    skillBelow9: boolean,
): string {
    const active = conditionList.filter(isConditionActive);
    return JSON.stringify([
        textOf(quick.keyword),
        quick.elementAttrId,
        quick.weaponCatId,
        quick.avatarLevel,
        textOf(quick.levelMin),
        textOf(quick.levelMax),
        skillBelow9,
        active.map((condition) => [
            condition.field,
            condition.operator,
            textOf(condition.value),
        ]),
        active.length ? relation : '',
    ]);
}

const draftSignature = computed(() =>
    signatureOf(quickFilter.value, conditions.value, conjunction.value, fixedSkillBelow9.value),
);
const draftDirty = computed(
    () => appliedFilter.value !== null && draftSignature.value !== appliedSignature.value,
);

function optionLabel(
    options: { value: string; label: string }[],
    value: string,
): string {
    return options.find((option) => option.value === value)?.label ?? value;
}

/** 已应用的筛选条件，用于结果区展示 */
const appliedChips = computed<string[]>(() => {
    const applied = appliedFilter.value;
    if (!applied) return [];

    const chips: string[] = [];
    const quick = applied.quick;
    const keyword = textOf(quick.keyword);
    const levelMinText = textOf(quick.levelMin);
    const levelMaxText = textOf(quick.levelMax);
    if (keyword) chips.push(`关键词：${keyword}`);
    if (quick.elementAttrId) {
        chips.push(`元素：${optionLabel(ELEMENT_OPTIONS, quick.elementAttrId)}`);
    }
    if (quick.weaponCatId) {
        chips.push(`武器：${optionLabel(WEAPON_OPTIONS, quick.weaponCatId)}`);
    }
    if (quick.avatarLevel) {
        chips.push(`星级：${optionLabel(RARITY_OPTIONS, quick.avatarLevel)}`);
    }
    if (levelMinText || levelMaxText) {
        chips.push(`等级：${levelMinText || '不限'} — ${levelMaxText || '不限'}`);
    }
    if (applied.fixedSkillBelow9) {
        chips.push('任一技能 < 9');
    }

    for (const condition of applied.conditions) {
        const field = fieldOf(condition.field);
        const rawValue = textOf(condition.value);
        const value = field.options ? optionLabel(field.options, rawValue) : rawValue;
        chips.push(`${field.label} ${OPERATOR_LABELS[condition.operator]} ${value}`);
    }

    if (applied.conditions.length > 1) {
        chips.push(
            applied.conjunction === 'and' ? '关系：全部满足' : '关系：任一满足',
        );
    }

    return chips;
});

const fileName = computed(() => filePath.value.replace(/^.*[\\/]/, ''));

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

function hasDraftConditions(): boolean {
    return Boolean(
        textOf(keyword.value) ||
            elementFilter.value ||
            weaponFilter.value ||
            rarityFilter.value ||
            textOf(levelMin.value) ||
            textOf(levelMax.value) ||
            fixedSkillBelow9.value ||
            activeConditionCount.value > 0,
    );
}

function optionalNumber(value: unknown): number | null {
    const trimmed = textOf(value);
    if (!trimmed) return null;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : null;
}

async function loadData() {
    loading.value = true;
    missing.value = false;
    error.value = '';
    try {
        const file = await api.readYsFile();
        if (!file) {
            characters.value = [];
            filePath.value = '';
            fileSize.value = 0;
            missing.value = true;
            logger.info('[YS] 未找到 data/其他/ys.json');
            return;
        }

        filePath.value = file.path;
        fileSize.value = file.size;
        const dataset = parseYsCharacters(file.content);
        characters.value = dataset.characters;
        logger.info(
            `[YS] 载入角色数据: ${dataset.characters.length} 个角色，文件 ${file.size} 字节`,
        );
    } catch (e) {
        const message = e instanceof Error ? e.message : String(e);
        characters.value = [];
        error.value = message;
        logger.warn(`[YS] 载入角色数据失败: ${message}`);
    } finally {
        loading.value = false;
    }
}

function applyFilters() {
    filterError.value = '';

    const min = optionalNumber(levelMin.value);
    const max = optionalNumber(levelMax.value);
    if (min !== null && max !== null && min > max) {
        filterError.value = '最低等级不能大于最高等级';
        return;
    }

    if (!hasDraftConditions()) {
        appliedFilter.value = null;
        appliedSignature.value = '';
        logger.info('[YS] 未设置筛选条件，显示全部角色');
        return;
    }

    const quick = { ...quickFilter.value };
    const conditionList = activeConditions.value.map((condition) => ({ ...condition }));
    const fixed = fixedSkillBelow9.value;
    appliedFilter.value = {
        quick,
        conditions: conditionList,
        conjunction: conjunction.value,
        fixedSkillBelow9: fixed,
    };
    appliedSignature.value = signatureOf(quick, conditionList, conjunction.value, fixed);
    logger.info(
        `[YS] 应用筛选: 自定义条件 ${conditionList.length} 条（${
            conjunction.value === 'and' ? '全部满足' : '任一满足'
        }），固定条件${fixed ? '已启用（任一技能 < 9）' : '未启用'}`,
    );
}

function resetFilters() {
    keyword.value = '';
    elementFilter.value = '';
    weaponFilter.value = '';
    rarityFilter.value = '';
    levelMin.value = '';
    levelMax.value = '';
    fixedSkillBelow9.value = false;
    conjunction.value = 'and';
    conditions.value = [];
    appliedFilter.value = null;
    appliedSignature.value = '';
    filterError.value = '';
    logger.info('[YS] 已重置筛选，显示全部角色');
}

function clearConditions() {
    conditions.value = [];
}

function onFieldChange(condition: FilterCondition) {
    const field = fieldOf(condition.field);
    const allowed = operatorsFor(field.kind);
    if (!allowed.includes(condition.operator)) {
        condition.operator = allowed[0]!;
    }
    condition.value = '';
}

function conditionOperators(condition: FilterCondition) {
    return operatorsFor(fieldOf(condition.field).kind);
}

function conditionOptions(condition: FilterCondition) {
    return fieldOf(condition.field).options ?? [];
}

function conditionKind(condition: FilterCondition) {
    return fieldOf(condition.field).kind;
}

function addCondition() {
    conditionSeq += 1;
    conditions.value.push({
        id: conditionSeq,
        field: 'levelCurrent',
        operator: 'gt',
        value: '',
    });
}

function removeCondition(id: number) {
    conditions.value = conditions.value.filter((condition) => condition.id !== id);
}

function weaponSummary(character: YsCharacter): string {
    if (!character.weapon) return '—';
    const weapon = character.weapon;
    return `${weapon.name} · ${weapon.weaponLevel}★ · Lv.${weapon.levelCurrent}/${weapon.maxLevel}`;
}

function hideIcon(event: Event) {
    const img = event.target as HTMLImageElement | null;
    if (img) img.style.display = 'none';
}

onMounted(loadData);
</script>

<template>
    <div class="ys-view">
        <div class="toolbar">
            <h2 class="page-title">角色图鉴</h2>
            <span class="count-info">
                <template v-if="appliedFilter">
                    筛选 {{ filteredCharacters.length }} / {{ characters.length }}
                </template>
                <template v-else>共 {{ characters.length }} 个角色</template>
            </span>
            <span v-if="filePath" class="file-info" :title="filePath">
                {{ fileName }} · {{ formatBytes(fileSize) }}
            </span>
            <button class="btn btn-ghost" :disabled="loading" @click="loadData">
                {{ loading ? '加载中…' : '重新加载' }}
            </button>
        </div>

        <div v-if="loading" class="empty-state">正在读取 data/其他/ys.json…</div>

        <div v-else-if="missing" class="empty-state">
            <div>没有找到 ys.json</div>
            <div class="hint">请将 ys.json 放到 data/其他/ 目录后重新加载</div>
            <button class="btn" @click="loadData">重新加载</button>
        </div>

        <div v-else-if="error" class="error-state">
            <div class="error-title">数据读取失败</div>
            <div class="error-message">{{ error }}</div>
            <button class="btn" @click="loadData">重新加载</button>
        </div>

        <template v-else>
            <form class="filter-panel" @submit.prevent="applyFilters">
                <div class="filter-row">
                    <input
                        v-model="keyword"
                        class="text-input search-input"
                        placeholder="搜索角色 / 武器 / 技能 / 圣遗物"
                    />
                    <select v-model="elementFilter" class="text-input filter-select">
                        <option value="">全部元素</option>
                        <option
                            v-for="option in ELEMENT_OPTIONS"
                            :key="option.value"
                            :value="option.value"
                        >
                            {{ option.label }}
                        </option>
                    </select>
                    <select v-model="weaponFilter" class="text-input filter-select">
                        <option value="">全部武器</option>
                        <option
                            v-for="option in WEAPON_OPTIONS"
                            :key="option.value"
                            :value="option.value"
                        >
                            {{ option.label }}
                        </option>
                    </select>
                    <select v-model="rarityFilter" class="text-input filter-select">
                        <option value="">全部星级</option>
                        <option
                            v-for="option in RARITY_OPTIONS"
                            :key="option.value"
                            :value="option.value"
                        >
                            {{ option.label }}
                        </option>
                    </select>
                    <label class="level-range">
                        <span>等级</span>
                        <input v-model="levelMin" class="text-input level-input" type="number" placeholder="最低" />
                        <span>—</span>
                        <input v-model="levelMax" class="text-input level-input" type="number" placeholder="最高" />
                    </label>
                    <span class="preset-group">
                        <span class="preset-label">固定条件</span>
                        <button
                            type="button"
                            class="preset-btn"
                            :class="{ active: fixedSkillBelow9 }"
                            title="三个可升级技能中任意一个低于 9 级"
                            @click="fixedSkillBelow9 = !fixedSkillBelow9"
                        >
                            任一技能 &lt; 9
                        </button>
                    </span>
                </div>

                <div class="custom-filter">
                    <div class="custom-head">
                        <span class="custom-title">
                            自定义筛选
                            <span class="muted">（有效 {{ activeConditionCount }} 条）</span>
                        </span>
                        <div class="conjunction">
                            <button
                                type="button"
                                :class="{ active: conjunction === 'and' }"
                                @click="conjunction = 'and'"
                            >
                                满足全部
                            </button>
                            <button
                                type="button"
                                :class="{ active: conjunction === 'or' }"
                                @click="conjunction = 'or'"
                            >
                                满足任一
                            </button>
                        </div>
                        <button type="button" class="btn btn-ghost" @click="addCondition">
                            + 添加条件
                        </button>
                        <button
                            v-if="conditions.length"
                            type="button"
                            class="btn btn-ghost"
                            @click="clearConditions"
                        >
                            清空条件
                        </button>
                    </div>

                    <div v-if="conditions.length" class="condition-list">
                        <div
                            v-for="condition in conditions"
                            :key="condition.id"
                            class="condition-row"
                        >
                            <select
                                v-model="condition.field"
                                class="text-input condition-field"
                                @change="onFieldChange(condition)"
                            >
                                <option
                                    v-for="field in FILTER_FIELDS"
                                    :key="field.key"
                                    :value="field.key"
                                >
                                    {{ field.label }}
                                </option>
                            </select>
                            <select
                                v-model="condition.operator"
                                class="text-input condition-op"
                            >
                                <option
                                    v-for="operator in conditionOperators(condition)"
                                    :key="operator"
                                    :value="operator"
                                >
                                    {{ OPERATOR_LABELS[operator] }}
                                </option>
                            </select>

                            <select
                                v-if="conditionOptions(condition).length"
                                v-model="condition.value"
                                class="text-input condition-value"
                            >
                                <option value="">请选择</option>
                                <option
                                    v-for="option in conditionOptions(condition)"
                                    :key="option.value"
                                    :value="option.value"
                                >
                                    {{ option.label }}
                                </option>
                            </select>
                            <input
                                v-else-if="
                                    conditionKind(condition) === 'number' ||
                                    conditionKind(condition) === 'numberList'
                                "
                                v-model="condition.value"
                                class="text-input condition-value"
                                type="number"
                                placeholder="数值"
                            />
                            <input
                                v-else
                                v-model="condition.value"
                                class="text-input condition-value"
                                placeholder="关键词"
                            />

                            <button
                                type="button"
                                class="btn btn-ghost"
                                @click="removeCondition(condition.id)"
                            >
                                删除
                            </button>
                        </div>
                    </div>
                    <div v-else class="custom-hint">
                        未添加自定义条件，可直接使用上方快捷筛选。
                    </div>

                    <div v-if="incompleteConditionCount > 0" class="condition-note">
                        有 {{ incompleteConditionCount }} 条条件未填写，点击「筛选」时将忽略。
                    </div>
                </div>

                <div class="filter-actions">
                    <button type="submit" class="btn btn-primary">筛选</button>
                    <button type="button" class="btn" @click="resetFilters">重置</button>
                    <span class="filter-hint">
                        设置条件后点击「筛选」生效；固定条件与其余条件同时满足；没有任何条件时显示全部角色。
                    </span>
                    <span v-if="draftDirty" class="dirty-hint">
                        条件已修改，点击「筛选」更新结果
                    </span>
                </div>
                <div v-if="filterError" class="filter-error">{{ filterError }}</div>
            </form>

            <div v-if="appliedFilter" class="applied-bar">
                <span class="applied-label">当前筛选</span>
                <span v-for="(chip, index) in appliedChips" :key="index" class="chip">
                    {{ chip }}
                </span>
                <button class="btn btn-ghost" @click="resetFilters">清除筛选</button>
            </div>

            <div class="result-bar">
                <span v-if="appliedFilter">
                    筛选出 {{ filteredCharacters.length }} / {{ characters.length }} 个角色
                </span>
                <span v-else>共 {{ characters.length }} 个角色</span>
                <span v-if="appliedFilter && appliedConditionCount > 0" class="muted">
                    自定义条件 {{ appliedConditionCount }} 条（{{
                        appliedFilter.conjunction === 'and' ? '全部满足' : '任一满足'
                    }}）
                </span>
            </div>

            <div v-if="characters.length === 0" class="empty-state">
                ys.json 中没有角色数据
            </div>

            <div v-else class="char-list">
                <div class="list-head">
                    <span>角色</span>
                    <span>元素</span>
                    <span>武器</span>
                    <span>星级</span>
                    <span>等级</span>
                    <span class="col-weapon">装备武器</span>
                    <span class="col-skills">技能等级</span>
                    <span></span>
                </div>

                <div v-if="filteredCharacters.length === 0" class="empty-state">
                    没有符合筛选条件的角色
                </div>

                <details
                    v-for="character in filteredCharacters"
                    :key="character.id"
                    class="char-item"
                >
                    <summary class="char-row">
                        <span class="char-main">
                            <span class="char-avatar">
                                <span class="avatar-fallback">{{ character.name.slice(0, 1) }}</span>
                                <img
                                    v-if="character.icon"
                                    :src="character.icon"
                                    :alt="character.name"
                                    loading="lazy"
                                    @error="hideIcon"
                                />
                            </span>
                            <span class="char-name-block">
                                <span class="char-name">{{ character.name }}</span>
                                <span class="char-id">#{{ character.id }}</span>
                            </span>
                        </span>
                        <span class="badge" :class="`element-${character.elementAttrId}`">
                            {{ elementLabel(character.elementAttrId) }}
                        </span>
                        <span class="cell">{{ weaponCatLabel(character.weaponCatId) }}</span>
                        <span class="cell rarity">{{ character.avatarLevel }}★</span>
                        <span class="cell level">
                            Lv.{{ character.levelCurrent }}<span class="muted">/{{ character.maxLevel }}</span>
                        </span>
                        <span class="cell col-weapon" :title="weaponSummary(character)">
                            {{ weaponSummary(character) }}
                        </span>
                        <span class="cell col-skills">
                            <span
                                v-for="skill in upgradableSkills(character)"
                                :key="skill.id"
                                class="skill-line"
                                :title="`${skill.name} Lv.${skill.levelCurrent}/${skill.maxLevel}`"
                            >
                                <span class="skill-name">{{ skill.name }}</span>
                                <span class="skill-level">Lv.{{ skill.levelCurrent }}</span>
                            </span>
                            <span v-if="!upgradableSkills(character).length" class="muted">—</span>
                        </span>
                        <span class="expander">▸</span>
                    </summary>

                    <div class="char-detail">
                        <div class="detail-columns">
                            <section class="detail-block">
                                <h4>
                                    技能
                                    <span class="muted">{{ upgradableSkills(character).length }}</span>
                                </h4>
                                <ul v-if="upgradableSkills(character).length" class="detail-list">
                                    <li v-for="skill in upgradableSkills(character)" :key="skill.id">
                                        <span>{{ skill.name }}</span>
                                        <span class="muted">
                                            Lv.{{ skill.levelCurrent }}/{{ skill.maxLevel }}
                                        </span>
                                    </li>
                                </ul>
                                <div v-else class="muted">暂无</div>
                            </section>

                            <section class="detail-block">
                                <h4>武器</h4>
                                <div v-if="character.weapon" class="weapon-detail">
                                    <span>{{ character.weapon.name }}</span>
                                    <span class="muted">
                                        {{ weaponCatLabel(character.weapon.weaponCatId) }} ·
                                        {{ character.weapon.weaponLevel }}★ ·
                                        Lv.{{ character.weapon.levelCurrent }}/{{ character.weapon.maxLevel }}
                                    </span>
                                </div>
                                <div v-else class="muted">暂无</div>
                            </section>

                            <section class="detail-block">
                                <h4>
                                    圣遗物
                                    <span class="muted">{{ character.reliquaryList.length }}</span>
                                </h4>
                                <ul v-if="character.reliquaryList.length" class="detail-list">
                                    <li v-for="reliquary in character.reliquaryList" :key="reliquary.id">
                                        <span>{{ reliquary.name }}</span>
                                        <span class="muted">
                                            {{ reliquaryCatLabel(reliquary.reliquaryCatId) }} ·
                                            {{ reliquary.reliquaryLevel }}★ ·
                                            Lv.{{ reliquary.levelCurrent }}/{{ reliquary.maxLevel }}
                                        </span>
                                    </li>
                                </ul>
                                <div v-else class="muted">暂无</div>
                            </section>
                        </div>

                        <div class="detail-meta">
                            <span>命座 {{ character.constellationNum }}</span>
                            <span>好感等级 {{ character.fetterLevel }}</span>
                            <span>等阶 {{ character.promoteLevel }}</span>
                            <span>ID {{ character.id }}</span>
                        </div>
                    </div>
                </details>
            </div>
        </template>
    </div>
</template>

<style scoped>
.ys-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    padding: 20px;
    box-sizing: border-box;
}

.toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0;
    border: none;
    background: transparent;
    min-height: auto;
    margin-bottom: 14px;
    flex-shrink: 0;
}

.page-title {
    margin: 0;
    font-size: 18px;
    color: #2c3e50;
}

.count-info {
    color: #6b7c8c;
    font-size: 13px;
}

.file-info {
    margin-left: auto;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #95a5a6;
    font-size: 12px;
}

.filter-panel {
    flex-shrink: 0;
    padding: 12px;
    background: #fff;
    border: 1px solid #e3e8ee;
    border-radius: 4px;
    margin-bottom: 10px;
}

.filter-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
}

.search-input {
    width: 220px;
}

.filter-select {
    width: 118px;
}

.level-range {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: #6b7c8c;
    font-size: 13px;
}

.level-input {
    width: 72px;
}

.preset-group {
    display: inline-flex;
    align-items: center;
    gap: 6px;
}

.preset-label {
    color: #6b7c8c;
    font-size: 13px;
}

.preset-btn {
    padding: 4px 10px;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    background: #fff;
    color: #4a5b6c;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
}

.preset-btn:hover {
    border-color: #2c3e50;
}

.preset-btn.active {
    background: #2c3e50;
    border-color: #2c3e50;
    color: #fff;
}

.custom-filter {
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px dashed #e3e8ee;
}

.custom-head {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
}

.custom-title {
    color: #2c3e50;
    font-size: 13px;
    font-weight: 600;
}

.conjunction {
    display: inline-flex;
    border: 1px solid #d0d0d0;
    border-radius: 4px;
    overflow: hidden;
}

.conjunction button {
    padding: 4px 10px;
    border: none;
    background: #fff;
    color: #6b7c8c;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
}

.conjunction button + button {
    border-left: 1px solid #d0d0d0;
}

.conjunction button.active {
    background: #2c3e50;
    color: #fff;
}

.condition-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 10px;
}

.condition-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
}

.condition-field {
    width: 132px;
}

.condition-op {
    width: 110px;
}

.condition-value {
    width: 150px;
}

.custom-hint {
    margin-top: 8px;
    color: #95a5a6;
    font-size: 12px;
}

.condition-note {
    margin-top: 6px;
    color: #b9770e;
    font-size: 12px;
}

.filter-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid #edf0f3;
}

.filter-hint {
    color: #95a5a6;
    font-size: 12px;
}

.dirty-hint {
    color: #b9770e;
    font-size: 12px;
}

.filter-error {
    margin-top: 8px;
    color: #c0392b;
    font-size: 12px;
}

.applied-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    flex-shrink: 0;
    margin-bottom: 8px;
    padding: 8px 10px;
    background: #f6f9fc;
    border: 1px solid #e3e8ee;
    border-radius: 4px;
}

.applied-label {
    color: #2c3e50;
    font-size: 12px;
    font-weight: 600;
}

.chip {
    padding: 2px 8px;
    border-radius: 10px;
    background: #e8f1fb;
    color: #2471a3;
    font-size: 12px;
}

.applied-bar .btn {
    margin-left: auto;
}

.result-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
    margin-bottom: 8px;
    color: #6b7c8c;
    font-size: 12px;
}

.char-list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: #fff;
    border: 1px solid #e3e8ee;
    border-radius: 4px;
}

.list-head,
.char-row {
    display: grid;
    grid-template-columns:
        minmax(180px, 1.3fr) 64px 82px 56px 96px minmax(150px, 1fr)
        minmax(200px, 1.5fr) 24px;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
}

.list-head {
    position: sticky;
    top: 0;
    z-index: 1;
    background: #fafbfc;
    border-bottom: 1px solid #edf0f3;
    color: #95a5a6;
    font-size: 12px;
    font-weight: 600;
}

.char-item {
    border-bottom: 1px solid #f0f3f5;
}

.char-item:last-child {
    border-bottom: none;
}

.char-item > summary {
    list-style: none;
    cursor: pointer;
    user-select: none;
}

.char-item > summary::-webkit-details-marker {
    display: none;
}

.char-row:hover {
    background: #f6f9fc;
}

.char-item[open] > .char-row {
    background: #f0f6fc;
}

.char-main {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
}

.char-avatar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: 50%;
    overflow: hidden;
    background: #eef2f6;
}

.avatar-fallback {
    color: #7f8c8d;
    font-size: 15px;
    font-weight: 600;
}

.char-avatar img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.char-name-block {
    display: flex;
    flex-direction: column;
    min-width: 0;
}

.char-name {
    color: #2c3e50;
    font-size: 14px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.char-id {
    color: #b2bec3;
    font-size: 11px;
}

.cell {
    color: #4a5b6c;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.col-skills {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
}

.skill-line {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    font-size: 12px;
    line-height: 1.4;
}

.skill-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.skill-level {
    flex-shrink: 0;
    color: #2471a3;
    font-family: 'Cascadia Code', Consolas, 'Courier New', monospace;
    font-size: 11px;
}

.rarity {
    color: #b9770e;
    font-weight: 600;
}

.level {
    font-family: 'Cascadia Code', Consolas, 'Courier New', monospace;
    font-size: 12px;
}

.muted {
    color: #95a5a6;
    font-weight: 400;
}

.badge {
    justify-self: start;
    padding: 1px 8px;
    border-radius: 3px;
    font-size: 12px;
    line-height: 20px;
}

.element-1 {
    background: #fdecea;
    color: #c0392b;
}

.element-2 {
    background: #eaf7ef;
    color: #1e8449;
}

.element-3 {
    background: #fdf3e3;
    color: #b9770e;
}

.element-4 {
    background: #eef7e6;
    color: #4d7c0f;
}

.element-5 {
    background: #f4ecf7;
    color: #7d3c98;
}

.element-6 {
    background: #e8f1fb;
    color: #2471a3;
}

.element-7 {
    background: #eaf6f8;
    color: #1a7f8c;
}

.expander {
    color: #95a5a6;
    font-size: 13px;
    text-align: center;
    transition: transform 0.15s;
}

.char-item[open] .expander {
    transform: rotate(90deg);
}

.char-detail {
    padding: 10px 14px 14px 58px;
    background: #fafbfc;
    border-top: 1px solid #edf0f3;
}

.detail-columns {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 18px;
}

.detail-block h4 {
    margin: 0 0 6px;
    color: #2c3e50;
    font-size: 13px;
}

.detail-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.detail-list li,
.weapon-detail {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    color: #4a5b6c;
    font-size: 12px;
}

.detail-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px dashed #e3e8ee;
    color: #95a5a6;
    font-size: 12px;
}

.error-state {
    margin: 40px auto;
    max-width: 560px;
    padding: 16px 18px;
    background: #fff;
    border: 1px solid #f0c9c4;
    border-radius: 4px;
    text-align: center;
}

.error-title {
    color: #c0392b;
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 6px;
}

.error-message {
    color: #6b7c8c;
    font-size: 13px;
    margin-bottom: 12px;
    word-break: break-word;
}

.empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #95a5a6;
    font-size: 13px;
    text-align: center;
}

@media (max-width: 1100px) {
    .list-head,
    .char-row {
        grid-template-columns: minmax(170px, 1.4fr) 60px 78px 50px 92px minmax(190px, 1.4fr) 24px;
    }

    .col-weapon {
        display: none;
    }
}

@media (max-width: 820px) {
    .ys-view {
        padding: 12px;
    }

    .search-input {
        width: 100%;
    }

    .list-head,
    .char-row {
        grid-template-columns: minmax(150px, 1.4fr) 58px 70px minmax(170px, 1.3fr) 24px;
    }

    .list-head > span:nth-child(4),
    .list-head > span:nth-child(5),
    .char-row > .rarity,
    .char-row > .level {
        display: none;
    }

    .detail-columns {
        grid-template-columns: 1fr;
        gap: 12px;
    }

    .char-detail {
        padding-left: 14px;
    }
}
</style>
