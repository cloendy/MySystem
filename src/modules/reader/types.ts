export interface Book {
    id: string;
    title: string;
    author: string;
    coverUrl: string;
    filePath: string;
    fileFormat: 'epub' | 'pdf' | 'mobi' | 'txt';
    /** 源文件内容 SHA-256（用于导入去重，旧数据可能为空） */
    hash: string;
    totalPages: number;
    currentPage: number;
    readingStatus: 'unread' | 'reading' | 'finished';
    /** DATETIME 文本，UTC，格式 `YYYY-MM-DD HH:MM:SS.mmm` */
    createdAt: string;
    updatedAt: string;
    deletedAt: string | null;
    version: number;
    deviceId: string;
}

export interface Annotation {
    id: string;
    bookId: string;
    text: string;
    note: string;
    page: number;
    position: string; // JSON 字符串
    color: string;
    /** DATETIME 文本，UTC，格式 `YYYY-MM-DD HH:MM:SS.mmm` */
    createdAt: string;
    updatedAt: string;
    deletedAt: string | null;
    version: number;
    deviceId: string;
}

export interface CreateBookInput {
    title: string;
    author?: string;
    coverUrl?: string;
    filePath: string;
    fileFormat: 'epub' | 'pdf' | 'mobi' | 'txt';
    hash?: string;
    totalPages?: number;
}

export interface UpdateBookInput {
    id: string;
    title?: string;
    author?: string;
    currentPage?: number;
    totalPages?: number;
    readingStatus?: 'unread' | 'reading' | 'finished';
}

export interface CreateAnnotationInput {
    bookId: string;
    text: string;
    note?: string;
    page: number;
    position?: string;
    color?: string;
}