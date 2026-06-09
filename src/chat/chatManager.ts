import { useCallback, useRef, useState } from "react";
import { requestCocreation, type CoCreateEdit } from "./cocreationClient";
import { createChatSessionId, saveChatTranscript } from "./chatPersistence";
import { createAssistantChatMessage, createUserChatMessage, type ChatMessage } from "./messageRepository";
import type { DraftKind, PromptContextPill, PromptContextRange } from "./promptContext";

export type ChatDraftEdit = CoCreateEdit & {
  id: string;
  sourceRange?: PromptContextRange;
  status: "pending" | "accepted" | "rejected";
};

export type SendChatPromptInput = {
  content: string;
  contextPills: PromptContextPill[];
  draftKind: DraftKind;
  selectedText?: string;
  sourcePath: string;
  text: string;
  title: string;
};

export function useChatManager({ onDraftEdits }: { onDraftEdits: (edits: ChatDraftEdit[]) => void }) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const sessionIdRef = useRef(createChatSessionId());

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
    const messagesWithUser = [...messages, userMessage];
    setMessages(messagesWithUser);
    void persistChat(messagesWithUser, input, sessionIdRef.current, setError);

    try {
      const response = await requestCocreation({
        sourcePath: input.sourcePath,
        title: input.title,
        content: input.content,
        contextItems: input.contextPills,
        draftKind: input.draftKind,
        userInput,
        selectedText: input.selectedText ?? "",
      });
      const messagesWithAssistant = [...messagesWithUser, createAssistantChatMessage(response.reply)];
      setMessages(messagesWithAssistant);
      void persistChat(messagesWithAssistant, input, sessionIdRef.current, setError);
      onDraftEdits(createPendingDraftEdits(response.edits, input.contextPills));
      return true;
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
      return false;
    } finally {
      setPending(false);
    }
  }, [messages, onDraftEdits, pending]);

  return {
    error,
    messages,
    pending,
    sendPrompt,
    setError,
  };
}

async function persistChat(
  messages: ChatMessage[],
  input: SendChatPromptInput,
  sessionId: string,
  setError: (error: string) => void,
) {
  try {
    await saveChatTranscript({
      messages,
      sessionId,
      sourcePath: input.sourcePath,
      title: input.title,
    });
  } catch (error) {
    setError(error instanceof Error ? error.message : String(error));
  }
}

function createPendingDraftEdits(edits: CoCreateEdit[], contextPills: PromptContextPill[]): ChatDraftEdit[] {
  const createdAt = Date.now();
  const selectedRangePill = contextPills.find((pill) => pill.kind === "selection" && pill.range);
  return edits.map((edit, index) => ({
    ...edit,
    id: `edit-${createdAt}-${index}`,
    sourceRange: selectedRangePill?.value.trim() === edit.target.trim() ? selectedRangePill.range : undefined,
    status: "pending" as const,
  }));
}
