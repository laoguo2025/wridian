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
  const projectIdRef = useRef("");
  const loadSeqRef = useRef(0);

  const resetChatSession = useCallback(() => {
    sessionIdRef.current = createChatSessionId();
    parentSessionIdRef.current = undefined;
    forkedFromMessageIdRef.current = undefined;
    messagesRef.current = [];
    setMessages([]);
  }, []);

  const stopActivePrompt = useCallback(() => {
    const requestId = activeRequestIdRef.current;
    if (!requestId) return;
    activeRequestIdRef.current = "";
    pendingRef.current = false;
    setPending(false);
    void abortCocreation(requestId).catch((requestError) => {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
    });
  }, []);

  const switchProjectChat = useCallback(async (projectId?: string) => {
    const nextProjectId = projectId?.trim() ?? "";
    const loadSeq = loadSeqRef.current + 1;
    loadSeqRef.current = loadSeq;
    projectIdRef.current = nextProjectId;
    stopActivePrompt();
    setError("");
    resetChatSession();

    try {
      const continuity = await loadChatContinuity(nextProjectId);
      if (loadSeqRef.current !== loadSeq) return;
      if (!continuity.sessionId || !continuity.messages.length) {
        resetChatSession();
        return;
      }
      sessionIdRef.current = continuity.sessionId;
      parentSessionIdRef.current = continuity.parentSessionId ?? undefined;
      forkedFromMessageIdRef.current = continuity.forkedFromMessageId ?? undefined;
      messagesRef.current = continuity.messages;
      setMessages(continuity.messages);
    } catch {
      if (loadSeqRef.current === loadSeq) {
        resetChatSession();
        setError("");
      }
    }
  }, [resetChatSession, stopActivePrompt]);

  useEffect(() => {
    void switchProjectChat("");
  }, [switchProjectChat]);

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
      projectIdRef.current,
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
        projectIdRef.current,
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

  const updateMessageText = useCallback((messageId: string, text: string, snapshot: ChatContinuitySnapshot) => {
    const nextText = text.trim();
    if (!nextText) return false;
    const targetMessage = messagesRef.current.find((message) => message.id === messageId);
    if (!targetMessage) return false;
    const nextMessages = messagesRef.current.map((message) =>
      message.id === messageId ? { ...message, text: nextText } : message,
    );
    messagesRef.current = nextMessages;
    setMessages(nextMessages);
    void saveChatTranscript({
      activeContext: buildMessageEditActiveContext({
        editedMessage: { ...targetMessage, text: nextText },
        messages: nextMessages,
        sessionId: sessionIdRef.current,
        snapshot,
      }),
      forkedFromMessageId: forkedFromMessageIdRef.current,
      messages: nextMessages,
      parentSessionId: parentSessionIdRef.current,
      projectId: projectIdRef.current,
      sessionId: sessionIdRef.current,
      sourcePath: snapshot.sourcePath,
      title: snapshot.title,
    }).catch((persistError) => {
      setError(persistError instanceof Error ? persistError.message : String(persistError));
    });
    return true;
  }, []);

  const stopPrompt = useCallback(() => {
    stopActivePrompt();
    setError("");
  }, [stopActivePrompt]);

  return {
    error,
    messages,
    pending,
    sendPrompt,
    setError,
    stopPrompt,
    switchProjectChat,
    updateMessageText,
  };
}

async function persistChat(
  messages: ChatMessage[],
  input: SendChatPromptInput,
  projectId: string,
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
      projectId,
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

function buildMessageEditActiveContext({
  editedMessage,
  messages,
  sessionId,
  snapshot,
}: {
  editedMessage: ChatMessage;
  messages: ChatMessage[];
  sessionId: string;
  snapshot: ChatContinuitySnapshot;
}): ActiveChatContext {
  const currentFragment = compactPlainText(snapshot.selectedText || snapshot.content, 900);
  const lastUserIntent = editedMessage.role === "user" ? compactPlainText(editedMessage.text, 320) : "修订了一条 Wridian 回复。";
  const lastJudgment = editedMessage.role === "assistant" ? compactPlainText(editedMessage.text, 420) : "用户消息已在对话气泡中修订。";
  const nextSuggestions = editedMessage.role === "user"
    ? ["按修订后的提问继续", "检查上下文是否仍匹配", "必要时重新发送本轮请求"]
    : ["基于修订后的回复继续", "对照正文确认修改方向", "把稳定判断沉淀到作品记忆"];
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
    return ["确认或拒绝正文改动", "换一个改法继续尝试", "继续细化角色口吻和节奏"];
  }
  if (assistantReply.includes("建议") || assistantReply.includes("可以")) {
    return ["选择一个建议继续展开", "回到当前片段做局部改写", "要求 Wridian 给出可确认 edits"];
  }
  return ["继续当前写作方向", "回到当前片段", "换一个方向尝试"];
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
