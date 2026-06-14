import { useCallback, useEffect, useRef, useState } from "react";
import {
  abortCocreation,
  requestCocreation,
  type CoCreateEdit,
} from "./cocreationClient";
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

type DraftEditCandidate = CoCreateEdit & {
  sourceRange?: PromptContextRange;
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

export function useChatManager({
  onDraftEdits,
  onWorkspaceChanged,
}: {
  onDraftEdits: (edits: ChatDraftEdit[], autoApply: boolean) => void;
  onWorkspaceChanged?: () => void;
}) {
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
    } catch (error) {
      if (loadSeqRef.current === loadSeq) {
        resetChatSession();
        setError(`对话续接读取失败：${error instanceof Error ? error.message : String(error)}`);
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
    const baseMessages = messagesRef.current;
    const messagesWithUser = [...baseMessages, userMessage];
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
      const assistantReply = response.reply;
      const messagesWithAssistant = [
        ...messagesWithContextStatus,
        createAssistantChatMessage(assistantReply, response.fileOperations),
      ];
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
          assistantReply,
          input,
          messages: messagesWithAssistant,
          sessionId: sessionIdRef.current,
        }),
      );
      const fileOperations = response.fileOperations;
      const draftEdits = fileOperations.length || isExplicitNewDocumentRequest(input.text) ? [] : response.edits;
      const pendingDraftEdits = createPendingDraftEdits(draftEdits, input);
      onDraftEdits(pendingDraftEdits, pendingDraftEdits.length > 0);
      if (fileOperations.some((operation) => operation.ok)) {
        onWorkspaceChanged?.();
      }
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
  }, [onDraftEdits, onWorkspaceChanged]);

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

  const truncateAfterMessage = useCallback((messageId: string) => {
    const index = messagesRef.current.findIndex((message) => message.id === messageId);
    if (index < 0) return false;
    const nextMessages = messagesRef.current.slice(0, index);
    messagesRef.current = nextMessages;
    setMessages(nextMessages);
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
    truncateAfterMessage,
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

function createPendingDraftEdits(edits: CoCreateEdit[], input: SendChatPromptInput): ChatDraftEdit[] {
  const createdAt = Date.now();
  const selectedRangePill = input.contextPills.find((pill) => pill.kind === "selection" && pill.range);
  const candidates = expandRepeatedLiteralEdits(edits, input, selectedRangePill);
  return candidates.map((edit, index) => {
    const selectedSourceRange = edit.sourceRange ?? (selectedRangePill?.value.trim() === edit.target.trim()
      ? selectedRangePill.range
      : undefined);
    const openingFallback = selectedSourceRange
      ? null
      : createOpeningRewriteFallback(edit, input, selectedRangePill);
    return {
      ...edit,
      id: `edit-${createdAt}-${index}`,
      sourceRange: selectedSourceRange ?? openingFallback?.sourceRange,
      status: "pending" as const,
      target: openingFallback?.target ?? edit.target,
    };
  });
}

function expandRepeatedLiteralEdits(
  edits: CoCreateEdit[],
  input: SendChatPromptInput,
  selectedRangePill?: PromptContextPill,
): DraftEditCandidate[] {
  if (!isAllOccurrencesEditIntent(input.text)) return edits;
  const selectedRange = selectedRangePill?.range;
  const selectedValue = selectedRangePill?.value.trim() ? selectedRangePill.value : "";
  const useSelectionScope = Boolean(shouldScopeEditToSelection(input.text) && selectedValue && selectedRange);
  const content = useSelectionScope ? selectedValue : input.content;
  const baseOffset = useSelectionScope && selectedRange ? selectedRange.start : 0;
  return edits.flatMap((edit) => {
    const target = edit.target.trim();
    if (!canExpandRepeatedLiteralEdit(edit, target)) return [edit];
    const ranges = findLiteralRanges(content, target);
    if (ranges.length <= 1) return [edit];
    return ranges.map((range) => ({
      ...edit,
      sourceRange: { start: baseOffset + range.start, end: baseOffset + range.end },
      target,
    }));
  });
}

function createOpeningRewriteFallback(
  edit: CoCreateEdit,
  input: SendChatPromptInput,
  selectedRangePill?: PromptContextPill,
): { sourceRange: PromptContextRange; target: string } | null {
  if (hasUniqueOccurrence(input.content, edit.target)) return null;

  if (selectedRangePill?.range && isRewriteIntent(input.text)) {
    const target = input.content.slice(selectedRangePill.range.start, selectedRangePill.range.end);
    if (target.trim()) {
      return { sourceRange: selectedRangePill.range, target };
    }
  }

  if (!isOpeningRewriteIntent(input.text)) return null;
  const openingRange = findOpeningRewriteRange(input.content, edit.target.length);
  if (!openingRange) return null;
  const target = input.content.slice(openingRange.start, openingRange.end);
  return target.trim() ? { sourceRange: openingRange, target } : null;
}

function isRewriteIntent(text: string) {
  return /重写|改写|润色|修改|优化|修正|增强|改成|改为|调整/.test(text);
}

function isOpeningRewriteIntent(text: string) {
  return isRewriteIntent(text) && /开头|开场|开篇|开幕|开局|开头段落|开场段落|第一段|前几段/.test(text);
}

function hasUniqueOccurrence(content: string, target: string) {
  if (!target) return false;
  const first = content.indexOf(target);
  if (first < 0) return false;
  return content.indexOf(target, first + Math.max(1, target.length)) < 0;
}

function findOpeningRewriteRange(content: string, modelTargetLength: number): PromptContextRange | null {
  if (!content.trim()) return null;
  const start = 0;
  const desiredLength = Math.min(1200, Math.max(180, modelTargetLength || 520));
  const minimumParagraphLength = Math.min(desiredLength, 240);
  const blankLinePattern = /\n\s*\n/g;
  let blankLineMatch: RegExpExecArray | null;
  while ((blankLineMatch = blankLinePattern.exec(content))) {
    const end = blankLineMatch.index;
    if (end - start >= minimumParagraphLength) {
      return { start, end };
    }
  }

  const desiredEnd = Math.min(content.length, desiredLength);
  const maxEnd = Math.min(content.length, Math.max(desiredEnd, 720));
  const lineBreak = content.indexOf("\n", desiredEnd);
  const end = lineBreak >= 0 && lineBreak + 1 <= maxEnd ? lineBreak + 1 : maxEnd;
  return end > start ? { start, end } : null;
}

function isExplicitNewDocumentRequest(text: string) {
  const normalized = normalizeIntentText(text);
  const hasCreateIntent = /新建|创建|新增|生成|写入|保存|放到|放在/.test(normalized);
  const hasDocumentTarget = /文档|文件|稿件|作品库|知识库|md|markdown|docx|txt/.test(normalized);
  return hasCreateIntent && hasDocumentTarget;
}

function normalizeIntentText(text: string) {
  return text.trim().replace(/\s+/g, "");
}

function findLiteralRanges(content: string, target: string) {
  const ranges: Array<{ start: number; end: number }> = [];
  if (!target) return ranges;
  let index = content.indexOf(target);
  while (index >= 0) {
    ranges.push({ start: index, end: index + target.length });
    index = content.indexOf(target, index + target.length);
  }
  return ranges;
}

function isAllOccurrencesEditIntent(text: string) {
  const normalized = normalizeIntentText(text);
  return /全部|全都|都|所有|统一|每个|每处|所有出现处|出现处/.test(normalized);
}

function shouldScopeEditToSelection(text: string) {
  const normalized = normalizeIntentText(text);
  return /选中|选区|这段|这几段|这部分|这一段|刚才划选|我划选/.test(normalized);
}

function canExpandRepeatedLiteralEdit(edit: CoCreateEdit, target: string) {
  return Boolean(
    target
      && target === edit.target
      && target.length <= 80
      && !target.includes("\n")
      && edit.replacement.trim()
      && edit.replacement.length <= 400
      && !edit.replacement.includes("\n\n"),
  );
}

function isAbortError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("对话已停止");
}
