import { logger } from '../../../platform/services/logger';

/** TXT 章节 */
export interface TxtChapter {
    title: string;
    content: string;
    lineStart: number;
}

/**
 * 解码 TXT 文件的字节内容：
 * BOM（UTF-16LE/BE、UTF-8）→ 严格 UTF-8 → 回退 GBK → 最后宽松 UTF-8。
 */
export function decodeText(bytes: Uint8Array): string {
    // BOM 检测
    if (bytes[0] === 0xff && bytes[1] === 0xfe) {
        return new TextDecoder('utf-16le').decode(bytes).replace(/^﻿/, '');
    }
    if (bytes[0] === 0xfe && bytes[1] === 0xff) {
        return new TextDecoder('utf-16be').decode(bytes).replace(/^﻿/, '');
    }
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
        return new TextDecoder('utf-8').decode(bytes).replace(/^﻿/, '');
    }

    // 尝试严格 UTF-8
    try {
        return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
        // 回退 GBK
        try {
            const text = new TextDecoder('gbk').decode(bytes);
            logger.info('[TXT] 使用 GBK 解码');
            return text;
        } catch {
            return new TextDecoder('utf-8').decode(bytes);
        }
    }
}

/**
 * 判断一行是否为章节标题：
 * 以「第」开头、包含「章/节/回」、长度不超过 40。
 */
export function isChapterHeading(line: string): boolean {
    const trimmed = line.trim();
    return (
        trimmed.startsWith('第') &&
        /第[一二三四五六七八九十百千万零〇\d]+[章节回]/.test(trimmed) &&
        trimmed.length > 0 &&
        trimmed.length < 40
    );
}

/**
 * 按章节标题拆分正文。
 *
 * 注意：只有当前章节已有正文时才会收录上一章，因此文件开头连续的卷名/目录标题
 * 会被合并进同一个章节，导入端与阅读端必须共用本函数，否则章节数会不一致。
 */
export function parseChapters(text: string): TxtChapter[] {
    const lines = text.split(/\r?\n/);
    const result: TxtChapter[] = [];
    let currentTitle = '序章';
    let currentLines: string[] = [];
    let currentStart = 0;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];

        if (isChapterHeading(line)) {
            const hasContent = currentLines.some((l) => l.trim().length > 0);
            if (hasContent) {
                result.push({
                    title: currentTitle,
                    content: currentLines.join('\n'),
                    lineStart: currentStart,
                });
            }
            currentTitle = line.trim();
            currentLines = [];
            currentStart = i;
        } else {
            currentLines.push(line);
        }
    }

    const hasContent = currentLines.some((l) => l.trim().length > 0);
    if (hasContent || result.length === 0) {
        result.push({
            title: currentTitle,
            content: currentLines.join('\n'),
            lineStart: currentStart,
        });
    }

    return result;
}

/** 统计 TXT 章节数（与阅读器完全一致的解析规则），至少返回 1 */
export function countChapters(bytes: Uint8Array): number {
    const chapters = parseChapters(decodeText(bytes));
    return chapters.length > 0 ? chapters.length : 1;
}
