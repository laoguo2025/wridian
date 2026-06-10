import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage } from "./messageRepository";

export type SaveChatTranscriptRequest = {
  messages: ChatMessage[];
  sessionId: string;
  sourcePath: string;
  title: string;
};

export type SaveChatTranscriptResponse = {
  path: string;
};

export type SaveChatKnowledgeCardRequest = {
  assistantMessage: string;
  cardTitle?: string;
  contextPills?: Array<{ label: string; value: string }>;
  sessionId: string;
  sourcePath: string;
  title: string;
  userMessage?: string;
};

export type SaveChatKnowledgeCardResponse = {
  path: string;
  title: string;
};

export function createChatSessionId() {
  return `chat-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export async function saveChatTranscript(request: SaveChatTranscriptRequest) {
  return invoke<SaveChatTranscriptResponse>("wridian_save_chat_transcript", {
    input: request,
  });
}

export async function saveChatKnowledgeCard(request: SaveChatKnowledgeCardRequest) {
  return invoke<SaveChatKnowledgeCardResponse>("wridian_save_chat_knowledge_card", {
    input: request,
  });
}
