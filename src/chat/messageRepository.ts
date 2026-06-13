import type { CoCreateFileOperation } from "./cocreationClient";
import type { PromptContextLoadStatus, PromptContextPill } from "./promptContext";

export type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  text: string;
  createdAt?: number;
  selectedText?: string;
  contextPills?: PromptContextPill[];
  contextLoadStatus?: PromptContextLoadStatus[];
  fileOperations?: CoCreateFileOperation[];
};

function createChatMessageId(prefix: ChatMessage["role"]) {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function createUserChatMessage(params: {
  contextPills: PromptContextPill[];
  selectedText?: string;
  text: string;
}): ChatMessage {
  const selectedText = (params.selectedText ?? "").trim();
  return {
    id: createChatMessageId("user"),
    role: "user",
    text: params.text,
    createdAt: Date.now(),
    selectedText: selectedText || undefined,
    contextPills: params.contextPills.length ? params.contextPills : undefined,
  };
}

export function createAssistantChatMessage(text: string, fileOperations: CoCreateFileOperation[] = []): ChatMessage {
  return {
    id: createChatMessageId("assistant"),
    role: "assistant",
    text,
    createdAt: Date.now(),
    fileOperations: fileOperations.length ? fileOperations : undefined,
  };
}

export function attachContextLoadStatus(message: ChatMessage, status: PromptContextLoadStatus[]): ChatMessage {
  if (!status.length) {
    return message;
  }
  return {
    ...message,
    contextLoadStatus: status,
  };
}

export function restorePromptPillsFromMessage(message: ChatMessage): PromptContextPill[] {
  if (message.contextPills?.length) {
    return message.contextPills;
  }
  if (!message.selectedText) {
    return [];
  }
  return [{ id: "message-context", kind: "selection", label: "上下文", value: message.selectedText }];
}

export function findPreviousUserMessage(messages: ChatMessage[], beforeIndex: number) {
  return [...messages.slice(0, beforeIndex)].reverse().find((message) => message.role === "user");
}
