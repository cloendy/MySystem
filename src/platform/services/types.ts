export interface BaseDocument {
    id: string;
    type: string;
    title: string;
    createdAt: number;
    updatedAt: number;
    deletedAt: number | null;
    version: number;
    deviceId: string;
}

export interface CreateDocumentInput {
    type: string;
    title: string;
    deviceId: string;
}

export interface UpdateDocumentInput {
    id: string;
    title?: string;
    deviceId: string;
}