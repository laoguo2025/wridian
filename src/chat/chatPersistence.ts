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

export function createChatSessionId() {
  return `chat-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export async function saveChatTranscript(request: SaveChatTranscriptRequest) {
  return invoke<SaveChatTranscriptResponse>("wridian_save_chat_transcript", {
    input: request,
  });
}
