export type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  text: string;
  selectedText?: string;
  contextPills?: PromptContextPill[];
};

export type PromptContextPill = {
  id: string;
  label: string;
  value: string;
};

export type PromptSuggestion = {
  id: string;
  label: string;
  detail: string;
  insertText: string;
  kind: "context" | "command";
};

function createChatMessageId(prefix: ChatMessage["role"]) {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function serializePromptContextPills(pills: PromptContextPill[]) {
  return pills.map((pill) => `【${pill.label}】\n${pill.value}`).join("\n\n").trim();
}

export function createUserChatMessage(params: {
  contextPills: PromptContextPill[];
  selectedText?: string;
  text: string;
}): ChatMessage {
  const selectedText = (params.selectedText ?? serializePromptContextPills(params.contextPills)).trim();
  return {
    id: createChatMessageId("user"),
    role: "user",
    text: params.text,
    selectedText: selectedText || undefined,
    contextPills: params.contextPills.length ? params.contextPills : undefined,
  };
}

export function createAssistantChatMessage(text: string): ChatMessage {
  return {
    id: createChatMessageId("assistant"),
    role: "assistant",
    text,
  };
}

export function restorePromptPillsFromMessage(message: ChatMessage): PromptContextPill[] {
  if (message.contextPills?.length) {
    return message.contextPills;
  }
  if (!message.selectedText) {
    return [];
  }
  return [{ id: "message-context", label: "上下文", value: message.selectedText }];
}

export function findPreviousUserMessage(messages: ChatMessage[], beforeIndex: number) {
  return [...messages.slice(0, beforeIndex)].reverse().find((message) => message.role === "user");
}
