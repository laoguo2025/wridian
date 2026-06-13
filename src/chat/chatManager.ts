import { useCallback, useEffect, useRef, useState } from "react";
import {
  abortCocreation,
  applyChatFileOperations,
  requestCocreation,
  type CoCreateEdit,
  type CoCreateFileOperationDraft,
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

export type LocalLiteralReplacePlan = {
  edits: ChatDraftEdit[];
  from: string;
  to: string;
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

    const localLiteralReplace = createLocalLiteralReplacePlan(input);
    if (localLiteralReplace) {
      const assistantReply = localLiteralReplace.edits.length
        ? `已在当前打开文件中找到 ${localLiteralReplace.edits.length} 处「${localLiteralReplace.from}」，将精确替换为「${localLiteralReplace.to}」。请在正文内联 diff 中确认后写入。`
        : `当前正文或选区里没有找到「${localLiteralReplace.from}」，未生成修改。`;
      const messagesWithAssistant = [...messagesWithUser, createAssistantChatMessage(assistantReply)];
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
      if (localLiteralReplace.edits.length) {
        onDraftEdits(localLiteralReplace.edits, true);
      }
      activeRequestIdRef.current = "";
      pendingRef.current = false;
      setPending(false);
      return true;
    }

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
      let fileOperations = response.fileOperations;
      let draftEdits = response.edits;
      if (!fileOperations.length) {
        const localFileOperation = createLocalWriteFileOperationFallback(input, assistantReply);
        if (localFileOperation) {
          draftEdits = [];
          const applied = await applyChatFileOperations([localFileOperation], input.sourcePath);
          fileOperations = applied.fileOperations;
          const messagesWithAppliedOperation = messagesWithAssistant.map((message) =>
            message.id === messagesWithAssistant[messagesWithAssistant.length - 1]?.id
              ? { ...message, fileOperations: fileOperations.length ? fileOperations : undefined }
              : message,
          );
          messagesRef.current = messagesWithAppliedOperation;
          setMessages(messagesWithAppliedOperation);
          void persistChat(
            messagesWithAppliedOperation,
            input,
            projectIdRef.current,
            sessionIdRef.current,
            parentSessionIdRef.current,
            forkedFromMessageIdRef.current,
            setError,
            buildActiveContext({
              assistantReply,
              input,
              messages: messagesWithAppliedOperation,
              sessionId: sessionIdRef.current,
            }),
          );
        } else if (isExplicitNewDocumentRequest(input.text)) {
          draftEdits = [];
        }
      }
      onDraftEdits(createPendingDraftEdits(draftEdits, input), shouldAutoApplyDraftEdits(input.text));
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

function createPendingDraftEdits(edits: CoCreateEdit[], input: SendChatPromptInput): ChatDraftEdit[] {
  const createdAt = Date.now();
  const selectedRangePill = input.contextPills.find((pill) => pill.kind === "selection" && pill.range);
  return edits.map((edit, index) => {
    const selectedSourceRange = selectedRangePill?.value.trim() === edit.target.trim()
      ? selectedRangePill.range
      : undefined;
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

function createLocalWriteFileOperationFallback(
  input: SendChatPromptInput,
  assistantReply: string,
): CoCreateFileOperationDraft | null {
  if (!isExplicitNewDocumentRequest(input.text)) return null;
  const content = extractLocalWriteFileContent(assistantReply).trim();
  if (!content || content.length < 12) return null;
  if (!looksLikeStandaloneDocumentBody(content)) return null;
  const filename = inferRequestedDocumentFilename(input.text) ?? "新建文档";
  return {
    action: "writeFile",
    library: inferRequestedLibrary(input.text),
    path: ensureEditableExtension(filename),
    content,
  };
}

function isExplicitNewDocumentRequest(text: string) {
  const normalized = normalizeIntentText(text);
  const hasCreateIntent = /新建|创建|新增|生成|写入|保存|放到|放在/.test(normalized);
  const hasDocumentTarget = /文档|文件|稿件|作品库|知识库|md|markdown|docx|txt/.test(normalized);
  return hasCreateIntent && hasDocumentTarget;
}

function inferRequestedLibrary(text: string): "works" | "knowledge" {
  return /知识库/.test(normalizeIntentText(text)) ? "knowledge" : "works";
}

function inferRequestedDocumentFilename(text: string) {
  const normalized = text.trim();
  const named = normalized.match(/(?:命名为|叫做|名为|文件名为|文档名为)\s*([^\s，。,.]{1,80})/);
  if (named?.[1]) return named[1].trim();
  const episodeFilename = inferNextEpisodeFilename(normalized);
  if (episodeFilename) return episodeFilename;
  const quotedTarget = normalized.match(
    /(?:新建|创建|新增|生成|写入|保存)(?:一个|一份|个|份)?(?:名为|叫做)?\s*[「『《“"]([^」』》”"]{1,80})[」』》”"]\s*(?:文档|文件|稿件|md|markdown|docx|txt)?/,
  );
  return quotedTarget?.[1]?.trim() || null;
}

function inferNextEpisodeFilename(text: string) {
  const normalized = normalizeIntentText(text);
  const directEpisodeMatches = [...normalized.matchAll(/(?:续写|写|生成|创建|新建)第([0-9一二三四五六七八九十百]+)集/g)];
  const directEpisode = directEpisodeMatches[directEpisodeMatches.length - 1]?.[1];
  if (directEpisode) return `第${directEpisode}集`;

  const episodeMatches = [...normalized.matchAll(/第([0-9一二三四五六七八九十百]+)集/g)];
  if (episodeMatches.length >= 2) {
    const targetEpisode = episodeMatches[episodeMatches.length - 1]?.[1];
    return targetEpisode ? `第${targetEpisode}集` : null;
  }
  return null;
}

function ensureEditableExtension(filename: string) {
  const cleanName = filename.replace(/[\\/:*?"<>|]/g, "").trim() || "新建文档";
  return /\.(md|markdown|txt|docx)$/i.test(cleanName) ? cleanName : `${cleanName}.md`;
}

function extractLocalWriteFileContent(text: string) {
  return text
    .replace(/这次模型没有返回可执行的文件树操作[\s\S]*?(?:目录。|目录|$)/g, "")
    .split(/\r?\n/)
    .filter((line) => !lineClaimsFileTreeWrite(line))
    .join("\n")
    .trim();
}

function looksLikeStandaloneDocumentBody(content: string) {
  const trimmed = content.trimStart();
  if (trimmed.startsWith("```")) return true;
  const firstLine = trimmed.split(/\r?\n/).find((line) => line.trim());
  if (!firstLine) return false;
  const line = firstLine.trimStart();
  return line.startsWith("# ") || line.startsWith("## ") || (/^第.+集/.test(line) && !line.includes("已"));
}

function lineClaimsFileTreeWrite(line: string) {
  const normalized = normalizeIntentText(line);
  if (!normalized) return false;
  const claimsDone = /已新建|已创建|已写入|已保存|新建为|创建为|写入到|保存到|新建|创建|写入|保存/.test(normalized);
  const mentionsFile = /works\/|knowledge\/|\.md|\.markdown|\.docx|\.txt|文件|文档/.test(normalized);
  return claimsDone && mentionsFile;
}

function normalizeIntentText(text: string) {
  return text.trim().replace(/\s+/g, "");
}

function createLocalLiteralReplacePlan(input: SendChatPromptInput): LocalLiteralReplacePlan | null {
  const parsed = parseLiteralReplaceIntent(input.text);
  if (!parsed) return null;
  const selectedRangePill = input.contextPills.find((pill) => pill.kind === "selection" && pill.range);
  const content = selectedRangePill?.value.trim() ? selectedRangePill.value : input.content;
  const baseOffset = selectedRangePill?.range ? selectedRangePill.range.start : 0;
  const ranges = findLiteralRanges(content, parsed.from);
  const createdAt = Date.now();
  return {
    edits: ranges.map((range, index) => ({
      id: `literal-replace-${createdAt}-${index}`,
      rationale: "按用户明确字面替换指令生成",
      replacement: parsed.to,
      sourceRange: { start: baseOffset + range.start, end: baseOffset + range.end },
      status: "pending" as const,
      target: parsed.from,
    })),
    from: parsed.from,
    to: parsed.to,
  };
}

function parseLiteralReplaceIntent(input: string): { from: string; to: string } | null {
  const normalized = input.trim().replace(/[“”]/g, "\"").replace(/[‘’]/g, "'");
  const patterns = [
    /^(?:请)?(?:把|将)\s*["'「『《]?(.+?)["'」』》]?\s*(?:全部|都|全都|统一)?\s*(?:改成|改为|换成|替换为|替换成)\s*["'「『《]?(.+?)["'」』》]?\s*$/,
    /^(?:请)?(?:将|把)?\s*["'「『《]?(.+?)["'」』》]?\s*(?:全部|都|全都|统一)?\s*(?:替换为|替换成|改成|改为|换成)\s*["'「『《]?(.+?)["'」』》]?\s*$/,
  ];
  for (const pattern of patterns) {
    const match = normalized.match(pattern);
    if (!match) continue;
    const from = cleanupLiteralReplaceTerm(match[1]);
    const to = cleanupLiteralReplaceTerm(match[2]);
    if (from && to && from !== to && from.length <= 80 && to.length <= 80) {
      return { from, to };
    }
  }
  return null;
}

function cleanupLiteralReplaceTerm(term: string) {
  return term
    .trim()
    .replace(/^(?:所有|全部|都|全都|正文中|文中|当前文件中|当前正文中|当前打开文件中|的)+/g, "")
    .replace(/(?:全部|都|全都|统一|这个词|这个名字|这个称呼|这几个字|这些字|所有出现处|出现处|在当前文件中|在当前正文中|在正文中|在文中)+$/g, "")
    .trim()
    .replace(/^["'「『《]+|["'」』》。！!，,、；;：:\s]+$/g, "")
    .trim();
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

function shouldAutoApplyDraftEdits(userInput: string) {
  const input = userInput.trim();
  if (!input) return false;
  if (/建议|方案|思路|为什么|怎么回事|原因|比较|评价|分析|解释/.test(input) && !/改成|换成|替换|删掉|删除|重写|改写/.test(input)) {
    return false;
  }
  if (/重写|改写|润色|修改|更改|替换|改成|换成|删掉|删除|合并|拆分|修正/.test(input)) {
    return true;
  }
  return /整理|优化|批量/.test(input) && /正文|稿件|原文|内容|这段|两段|选中|对白|句子|段落|修改|替换|删除/.test(input);
}

function isAbortError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes("对话已停止");
}
