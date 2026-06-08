import { useCallback, useState } from "react";
import { requestCocreation, type CoCreateEdit } from "./cocreationClient";
import { createAssistantChatMessage, createUserChatMessage, type ChatMessage } from "./messageRepository";
import type { PromptContextPill } from "./promptContext";

export type ChatDraftEdit = CoCreateEdit & {
  id: string;
  status: "pending" | "accepted" | "rejected";
};

export type SendChatPromptInput = {
  content: string;
  contextPills: PromptContextPill[];
  selectedText?: string;
  sourcePath: string;
  text: string;
  title: string;
};

export function useChatManager({ onDraftEdits }: { onDraftEdits: (edits: ChatDraftEdit[]) => void }) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  const sendPrompt = useCallback(async (input: SendChatPromptInput) => {
    const userInput = input.text.trim();
    if (!userInput || pending) return false;

    setPending(true);
    setError("");

    const userMessage = createUserChatMessage({
      contextPills: input.contextPills,
      selectedText: input.selectedText,
      text: userInput,
    });
    const selectedText = userMessage.selectedText ?? "";
    setMessages((current) => [...current, userMessage]);

    try {
      const response = await requestCocreation({
        sourcePath: input.sourcePath,
        title: input.title,
        content: input.content,
        userInput,
        selectedText,
      });
      setMessages((current) => [...current, createAssistantChatMessage(response.reply)]);
      onDraftEdits(createPendingDraftEdits(response.edits));
      return true;
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
      return false;
    } finally {
      setPending(false);
    }
  }, [onDraftEdits, pending]);

  return {
    error,
    messages,
    pending,
    sendPrompt,
    setError,
  };
}

function createPendingDraftEdits(edits: CoCreateEdit[]): ChatDraftEdit[] {
  const createdAt = Date.now();
  return edits.map((edit, index) => ({
    ...edit,
    id: `edit-${createdAt}-${index}`,
    status: "pending" as const,
  }));
}
