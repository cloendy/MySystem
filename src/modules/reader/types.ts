export interface Book {
    id: string;
    title: string;
    author: string;
    coverUrl: string;
    filePath: string;
    fileFormat: 'epub' | 'pdf' | 'mobi' | 'txt';
    totalPages: number;
    currentPage: number;
    readingStatus: 'unread' | 'reading' | 'finished';
    createdAt: number;
    updatedAt: number;
    deletedAt: number | null;
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
    createdAt: number;
    updatedAt: number;
    deletedAt: number | null;
    version: number;
    deviceId: string;
}

export interface CreateBookInput {
    title: string;
    author?: string;
    coverUrl?: string;
    filePath: string;
    fileFormat: 'epub' | 'pdf' | 'mobi' | 'txt';
    totalPages?: number;
}

export interface UpdateBookInput {
    id: string;
    title?: string;
    author?: string;
    currentPage?: number;
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