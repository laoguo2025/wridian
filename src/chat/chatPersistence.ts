import { invoke } from "@tauri-apps/api/core";
import type { ChatMessage } from "./messageRepository";

export type ActiveChatContext = {
  schemaVersion: 1;
  sessionId: string;
  currentWork: {
    source: string;
    title: string;
  };
  currentFragment: string;
  lastUserIntent: string;
  lastJudgment: string;
  nextSuggestions: string[];
  compactSummary: string;
  updatedAt: string;
};

export type SaveChatTranscriptRequest = {
  activeContext?: ActiveChatContext;
  forkedFromMessageId?: string;
  messages: ChatMessage[];
  parentSessionId?: string;
  projectId?: string;
  sessionId: string;
  sourcePath: string;
  title: string;
};

export type SaveChatTranscriptResponse = {
  path: string;
};

export type LoadChatContinuityResponse = {
  activeContext?: ActiveChatContext;
  forkedFromMessageId?: string | null;
  messages: ChatMessage[];
  parentSessionId?: string | null;
  sessionId: string;
  sourcePath: string;
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

export async function loadChatContinuity(projectId?: string) {
  return invoke<LoadChatContinuityResponse>("wridian_load_chat_continuity", {
    input: {
      projectId: projectId?.trim() || null,
    },
  });
}
