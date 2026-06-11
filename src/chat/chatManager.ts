import { useCallback, useEffect, useRef, useState } from "react";
import { abortCocreation, requestCocreation, type CoCreateEdit } from "./cocreationClient";
import {
  createChatSessionId,
  loadChatContinuity,
  saveChatTranscript,
  type ActiveChatContext,
} from "./chatPersistence";
import {
  attachContextLoadStatus,
  createAssistantChatMessage,
  createUserChatMessage,
  type ChatMessage,
} from "./messageRepository";
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
  requestId: string;
  selectedModelId?: string;
  selectedText?: string;
  sourcePath: string;
  text: string;
  title: string;
};

export type ChatContinuitySnapshot = {
  content: string;
  selectedText?: string;
  sourcePath: string;
  title: string;
};

export function useChatManager({ onDraftEdits }: { onDraftEdits: (edits: ChatDraftEdit[]) => void }) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const sessionIdRef = useRef(createChatSessionId());
  const parentSessionIdRef = useRef<string | undefined>(undefined);
  const forkedFromMessageIdRef = useRef<string | undefined>(undefined);
  const activeRequestIdRef = useRef("");
  const pendingRef = useRef(false);
  const messagesRef = useRef<ChatMessage[]>([]);

  useEffect(() => {
    let cancelled = false;
    void loadChatContinuity()
      .then((continuity) => {
        if (cancelled || !continuity.sessionId || !continuity.messages.length) return;
        sessionIdRef.current = continuity.sessionId;
        parentSessionIdRef.current = continuity.parentSessionId ?? undefined;
        forkedFromMessageIdRef.current = continuity.forkedFromMessageId ?? undefined;
        messagesRef.current = continuity.messages;
        setMessages(continuity.messages);
      })
      .catch(() => {
        if (!cancelled) setError("");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const sendPrompt = useCallback(async (input: SendChatPromptInput) => {
    const userInput = input.text.trim();
    if (!userInput || pendingRef.current) return false;

    const requestId = input.requestId;
    pendingRef.current = true;
    activeRequestIdRef.current = requestId;
    setPending(true);
    setError("");

    const userMessage = createUserChatMessage({
      contextPills: input.contextPills,
      selectedText: input.selectedText,
      text: userInput,
    });
    const messagesWithUser = [...messagesRef.current, userMessage];
    messagesRef.current = messagesWithUser;
    setMessages(messagesWithUser);
    void persistChat(
      messagesWithUser,
      input,
      sessionIdRef.current,
      parentSessionIdRef.current,
      forkedFromMessageIdRef.current,
      setError,
      buildActiveContext({
        assistantReply: "等待 Wridian 回复。",
        input,
        messages: messagesWithUser,
        sessionId: sessionIdRef.current,
      }),
    );

    try {
      const response = await requestCocreation({
        sourcePath: input.sourcePath,
        title: input.title,
        content: input.content,
        contextItems: input.contextPills,
        draftKind: input.draftKind,
        requestId,
        selectedModelId: input.selectedModelId,
        userInput,
        selectedText: input.selectedText ?? "",
      });
      if (activeRequestIdRef.current !== requestId) {
        return false;
      }
      const messagesWithContextStatus = messagesWithUser.map((message) =>
        message.id === userMessage.id ? attachContextLoadStatus(message, response.contextLoadStatus) : message,
      );
      const messagesWithAssistant = [...messagesWithContextStatus, createAssistantChatMessage(response.reply)];
      messagesRef.current = messagesWithAssistant;
      setMessages(messagesWithAssistant);
      void persistChat(
        messagesWithAssistant,
        input,
        sessionIdRef.current,
        parentSessionIdRef.current,
        forkedFromMessageIdRef.current,
        setError,
        buildActiveContext({
          assistantReply: response.reply,
          input,
          messages: messagesWithAssistant,
          sessionId: sessionIdRef.current,
        }),
      );
      onDraftEdits(createPendingDraftEdits(response.edits, input.contextPills));
      return true;
    } catch (requestError) {
      if (activeRequestIdRef.current !== requestId || isAbortError(requestError)) {
        return false;
      }
      setError(requestError instanceof Error ? requestError.message : String(requestError));
      return false;
    } finally {
      if (activeRequestIdRef.current === requestId) {
        activeRequestIdRef.current = "";
        pendingRef.current = false;
        setPending(false);
      }
    }
  }, [onDraftEdits]);

  const forkFromMessage = useCallback((messageId: string, snapshot: ChatContinuitySnapshot) => {
    const index = messages.findIndex((message) => message.id === messageId);
    if (index < 0) return false;
    const forkMessages = messages.slice(0, index + 1);
    const parentSessionId = sessionIdRef.current;
    const nextSessionId = createChatSessionId();
    sessionIdRef.current = nextSessionId;
    parentSessionIdRef.current = parentSessionId;
    forkedFromMessageIdRef.current = messageId;
    messagesRef.current = forkMessages;
    setMessages(forkMessages);
    void saveChatTranscript({
      activeContext: buildForkActiveContext({
        forkedFromMessage: forkMessages[index],
        messages: forkMessages,
        sessionId: nextSessionId,
        snapshot,
      }),
      forkedFromMessageId: messageId,
      messages: forkMessages,
      parentSessionId,
      sessionId: nextSessionId,
      sourcePath: snapshot.sourcePath,
      title: snapshot.title,
    }).catch((persistError) => {
      setError(persistError instanceof Error ? persistError.message : String(persistError));
    });
    return true;
  }, [messages]);

  const stopPrompt = useCallback(() => {
    const requestId = activeRequestIdRef.current;
    if (!requestId) return;
    activeRequestIdRef.current = "";
    pendingRef.current = false;
    setPending(false);
    setError("");
    void abortCocreation(requestId).catch((requestError) => {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
    });
  }, []);

  return {
    error,
    messages,
    pending,
    forkFromMessage,
    sendPrompt,
    setError,
    stopPrompt,
  };
}

async function persistChat(
  messages: ChatMessage[],
  input: SendChatPromptInput,
  sessionId: string,
  parentSessionId: string | undefined,
  forkedFromMessageId: string | undefined,
  setError: (error: string) => void,
  activeContext: ActiveChatContext,
) {
  try {
    await saveChatTranscript({
      activeContext,
      forkedFromMessageId,
      messages,
      parentSessionId,
      sessionId,
      sourcePath: input.sourcePath,
      title: input.title,
    });
  } catch (error) {
    setError(error instanceof Error ? error.message : String(error));
  }
}

function buildActiveContext({
  assistantReply,
  input,
  messages,
  sessionId,
}: {
  assistantReply: string;
  input: SendChatPromptInput;
  messages: ChatMessage[];
  sessionId: string;
}): ActiveChatContext {
  const currentFragment = compactPlainText(input.selectedText || input.content, 900);
  const lastUserIntent = compactPlainText(input.text, 320);
  const lastJudgment = compactPlainText(assistantReply, 420);
  const nextSuggestions = deriveNextSuggestions(input.text, assistantReply);
  return {
    schemaVersion: 1,
    sessionId,
    currentWork: {
      source: input.sourcePath || "未选择文件",
      title: input.title || "未选择文件",
    },
    currentFragment,
    lastUserIntent,
    lastJudgment,
    nextSuggestions,
    compactSummary: renderCompactSummary({
      currentFragment,
      lastJudgment,
      lastUserIntent,
      messageCount: messages.length,
      nextSuggestions,
      title: input.title || "未选择文件",
    }),
    updatedAt: new Date().toISOString(),
  };
}

function buildForkActiveContext({
  forkedFromMessage,
  messages,
  sessionId,
  snapshot,
}: {
  forkedFromMessage: ChatMessage;
  messages: ChatMessage[];
  sessionId: string;
  snapshot: ChatContinuitySnapshot;
}): ActiveChatContext {
  const currentFragment = compactPlainText(snapshot.selectedText || snapshot.content, 900);
  const lastUserIntent = "从一条回复分叉，尝试另一个修改方向。";
  const lastJudgment = compactPlainText(forkedFromMessage.text, 420);
  const nextSuggestions = ["沿新方向继续改写", "回到分叉点比较两个版本", "把更稳定的判断沉淀到作品记忆"];
  return {
    schemaVersion: 1,
    sessionId,
    currentWork: {
      source: snapshot.sourcePath || "未选择文件",
      title: snapshot.title || "未选择文件",
    },
    currentFragment,
    lastUserIntent,
    lastJudgment,
    nextSuggestions,
    compactSummary: renderCompactSummary({
      currentFragment,
      lastJudgment,
      lastUserIntent,
      messageCount: messages.length,
      nextSuggestions,
      title: snapshot.title || "未选择文件",
    }),
    updatedAt: new Date().toISOString(),
  };
}

function deriveNextSuggestions(userInput: string, assistantReply: string) {
  const lower = userInput.toLowerCase();
  if (lower.includes("继续") || userInput.includes("接着")) {
    return ["继续当前段落", "检查上一轮方向是否仍成立", "把必要约束写入作品记忆"];
  }
  if (userInput.includes("改") || userInput.includes("润色") || userInput.includes("重写")) {
    return ["确认或拒绝正文改动", "从本轮回复分叉另一个改法", "继续细化角色口吻和节奏"];
  }
  if (assistantReply.includes("建议") || assistantReply.includes("可以")) {
    return ["选择一个建议继续展开", "回到当前片段做局部改写", "要求 Wridian 给出可确认 edits"];
  }
  return ["继续当前写作方向", "回到当前片段", "换一个方向分叉尝试"];
}

function renderCompactSummary(input: {
  currentFragment: string;
  lastJudgment: string;
  lastUserIntent: string;
  messageCount: number;
  nextSuggestions: string[];
  title: string;
}) {
  return [
    "# 创作交接卡",
    "",
    `- 当前作品：${input.title}`,
    `- 当前片段：${input.currentFragment || "暂无"}`,
    `- 上次用户意图：${input.lastUserIntent || "暂无"}`,
    `- 上次判断：${input.lastJudgment || "暂无"}`,
    `- 已恢复轮次：${input.messageCount}`,
    "- 下一步建议：",
    ...input.nextSuggestions.map((suggestion) => `  - ${suggestion}`),
  ].join("\n");
}

function compactPlainText(text: string, maxChars: number) {
  return text.split(/\s+/).filter(Boolean).join(" ").slice(0, maxChars);
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

function isAbortError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("对话已停止");
}
