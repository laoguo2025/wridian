import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { WridianPromptEditor } from "./WridianPromptEditor";
import {
  findPreviousUserMessage,
  restorePromptPillsFromMessage,
  type ChatMessage,
} from "./messageRepository";
import type { PromptContextLoadStatus, PromptContextPill, PromptSuggestion } from "./promptContext";
import type { ProjectConfig, RelevantNote } from "./projectContext";
import type { ConfiguredModelStatus } from "../appTypes";
import {
  ContextIcon,
  CopyIcon,
  MessageEditCancelIcon,
  MessageEditIcon,
  MessageEditSubmitIcon,
  RetryIcon,
} from "../icons";

type ProjectMenuPosition = {
  left: number;
  top: number;
};

export function ChatPanel({
  activeModelLabel,
  configuredModels,
  error,
  messages,
  onCopy,
  onPromptChange,
  onPromptPillsChange,
  onImagePaste,
  onRemovePill,
  onRetry,
  onSelectRelevantNote,
  onSelectModel,
  onSelectSuggestion,
  onSelectProject,
  onStop,
  onSubmit,
  onUpdateMessageText,
  pending,
  projectError,
  projects,
  prompt,
  promptPills,
  promptSuggestions,
  relevantNotes,
  relevantNotesError,
  selectedProjectId,
  selectedModelId,
}: {
  activeModelLabel: string;
  configuredModels: ConfiguredModelStatus[];
  error: string;
  messages: ChatMessage[];
  onCopy: (text: string) => void;
  onPromptChange: (value: string) => void;
  onPromptPillsChange: (pills: PromptContextPill[]) => void;
  onImagePaste: (files: File[]) => void;
  onRemovePill: (id: string) => void;
  onRetry: (message: ChatMessage) => void;
  onSelectRelevantNote: (note: RelevantNote) => void;
  onSelectModel: (id: string) => void;
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  onSelectProject: (id: string) => void;
  onStop: () => void;
  onSubmit: () => void;
  onUpdateMessageText: (message: ChatMessage, text: string) => void;
  pending: boolean;
  projectError: string;
  projects: ProjectConfig[];
  prompt: string;
  promptPills: PromptContextPill[];
  promptSuggestions: PromptSuggestion[];
  relevantNotes: RelevantNote[];
  relevantNotesError: string;
  selectedProjectId: string;
  selectedModelId: string;
}) {
  const threadRef = useRef<HTMLDivElement | null>(null);
  const projectTriggerRef = useRef<HTMLButtonElement | null>(null);
  const projectMenuRef = useRef<HTMLDivElement | null>(null);
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const [projectMenuPosition, setProjectMenuPosition] = useState<ProjectMenuPosition | null>(null);
  const activeProjectName = projects.find((project) => project.id === selectedProjectId)?.name ?? "普通聊天";
  const projectOptions = [{ id: "", name: "普通聊天" }, ...projects];
  const canOpenProjectMenu = projects.length > 0;

  useEffect(() => {
    const thread = threadRef.current;
    if (!thread) return;
    const frame = window.requestAnimationFrame(() => {
      thread.scrollTop = thread.scrollHeight;
    });
    return () => window.cancelAnimationFrame(frame);
  }, [error, messages.length, pending]);

  useEffect(() => {
    if (!projectMenuOpen) return;
    const closeProjectMenu = (event: PointerEvent) => {
      const target = event.target instanceof Node ? event.target : null;
      if (
        target
        && (projectTriggerRef.current?.contains(target) || projectMenuRef.current?.contains(target))
      ) {
        return;
      }
      setProjectMenuOpen(false);
    };
    const closeProjectMenuOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setProjectMenuOpen(false);
    };
    window.addEventListener("pointerdown", closeProjectMenu, true);
    window.addEventListener("keydown", closeProjectMenuOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeProjectMenu, true);
      window.removeEventListener("keydown", closeProjectMenuOnEscape);
    };
  }, [projectMenuOpen]);

  const projectMenu = projectMenuOpen && projectMenuPosition ? createPortal(
    <div
      className="chat-project-menu"
      ref={projectMenuRef}
      role="listbox"
      aria-label="项目对话"
      style={{
        left: `${projectMenuPosition.left}px`,
        top: `${projectMenuPosition.top}px`,
      }}
    >
      {projectOptions.map((project) => (
        <button
          type="button"
          className={`chat-project-option ${project.id === selectedProjectId ? "active" : ""}`}
          key={project.id || "default-chat"}
          role="option"
          aria-selected={project.id === selectedProjectId}
          onClick={() => {
            onSelectProject(project.id);
            setProjectMenuOpen(false);
          }}
        >
          {project.name}
        </button>
      ))}
    </div>,
    document.body,
  ) : null;
  const latestUserMessageId = [...messages].reverse().find((message) => message.role === "user")?.id;

  return (
    <aside className="chat-panel" aria-label="对话区">
      <div className="chat-modebar">
        <button
          type="button"
          ref={projectTriggerRef}
          className="chat-project-trigger"
          aria-expanded={projectMenuOpen}
          aria-haspopup="listbox"
          onClick={() => {
            if (!canOpenProjectMenu) {
              setProjectMenuOpen(false);
              return;
            }
            const rect = projectTriggerRef.current?.getBoundingClientRect();
            setProjectMenuPosition(rect ? {
              left: Math.max(12, rect.left - 228),
              top: rect.top,
            } : null);
            setProjectMenuOpen((open) => !open);
          }}
          title={activeProjectName}
        >
            <span>{activeProjectName}</span>
          <span aria-hidden="true">▾</span>
        </button>
        {projectMenu}
      </div>
      {projectError ? <div className="chat-status error">{projectError}</div> : null}
      <div className="chat-thread" ref={threadRef}>
        {messages.length
          ? messages.map((message, index) => (
              <ChatTimelineItem
                current={message}
                key={message.id}
                previous={messages[index - 1]}
              >
                <ChatMessageView
                  message={message}
                  canModify={message.id === latestUserMessageId}
                  onCopy={onCopy}
                  onRetry={onRetry}
                  onUpdateMessageText={onUpdateMessageText}
                  userForRetry={findPreviousUserMessage(messages, index)}
                />
              </ChatTimelineItem>
            ))
          : null}
        {pending ? <div className="chat-status">正在回复。</div> : null}
        {error ? <div className="chat-status error">{error}</div> : null}
      </div>

      {relevantNotes.length || relevantNotesError ? (
        <section className="relevant-notes" aria-label="相关内容">
          <div className="relevant-notes-header">相关内容</div>
          {relevantNotesError ? <div className="relevant-notes-error">{relevantNotesError}</div> : null}
          {relevantNotes.map((note) => (
            <button
              className="relevant-note"
              key={note.path}
              onClick={() => onSelectRelevantNote(note)}
              title={note.reasons.join("；")}
              type="button"
            >
              <span className="relevant-note-title">
                <span className="relevant-note-kind">{note.kind === "knowledge" ? "知识卡" : "稿件"}</span>
                {note.title}
              </span>
              {note.snippet ? <small>{note.snippet}</small> : null}
              {note.reasons.length ? <em>{note.reasons.join("；")}</em> : null}
            </button>
          ))}
        </section>
      ) : null}

      <form
        className="prompt-bar"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        {promptPills.length ? (
          <div className="prompt-attachments" aria-label="已添加上下文">
            {promptPills.map((pill) => (
              <span className={`prompt-attachment ${pillClassName(pill)}`} key={pill.id}>
                <span className="prompt-attachment-kind">{pillKindLabel(pill)}</span>
                <span>{pill.label}</span>
                <button type="button" onClick={() => onRemovePill(pill.id)} aria-label={`移除${pill.label}`}>
                  ×
                </button>
              </span>
            ))}
          </div>
        ) : null}
        <WridianPromptEditor
          value={prompt}
          onChange={onPromptChange}
          onImagePaste={onImagePaste}
          onPillsChange={onPromptPillsChange}
          onSelectSuggestion={onSelectSuggestion}
          onSubmit={onSubmit}
          placeholder="与 Wridian 对话"
          suggestions={promptSuggestions}
        />
        <div className="prompt-footer">
          {configuredModels.length ? (
            <select
              className="prompt-model-select"
              value={selectedModelId}
              onChange={(event) => onSelectModel(event.currentTarget.value)}
              aria-label="切换模型"
              title={activeModelLabel || "切换模型"}
            >
              {configuredModels.map((model) => (
                <option value={model.id} key={model.id}>{model.label}</option>
              ))}
            </select>
          ) : (
            <span className="prompt-model-label" aria-label="当前模型">
              {activeModelLabel || "未配置模型"}
            </span>
          )}
          <button
            type={pending ? "button" : "submit"}
            className="prompt-send"
            aria-label={pending ? "停止" : "发送"}
            disabled={!pending && (!prompt.trim() && !promptPills.length)}
            onClick={pending ? onStop : undefined}
          >
            {pending ? "停止" : "发送"}
          </button>
        </div>
      </form>
    </aside>
  );
}

function ChatTimelineItem({
  children,
  current,
  previous,
}: {
  children: ReactNode;
  current: ChatMessage;
  previous?: ChatMessage;
}) {
  const currentTime = messageTime(current);
  const previousTime = previous ? messageTime(previous) : undefined;
  const showTime = !previousTime || currentTime - previousTime > 5 * 60 * 1000;
  return (
    <>
      {showTime ? <time className="chat-time-separator">{formatMessageTime(currentTime)}</time> : null}
      {children}
    </>
  );
}

function ChatMessageView({
  message,
  canModify,
  onCopy,
  onRetry,
  onUpdateMessageText,
  userForRetry,
}: {
  message: ChatMessage;
  canModify: boolean;
  onCopy: (text: string) => void;
  onRetry: (message: ChatMessage) => void;
  onUpdateMessageText: (message: ChatMessage, text: string) => void;
  userForRetry?: ChatMessage;
}) {
  const [contextOpen, setContextOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const [editing, setEditing] = useState(false);
  const [draftText, setDraftText] = useState(message.text);
  const contextButtonRef = useRef<HTMLButtonElement | null>(null);
  const contextPopoverRef = useRef<HTMLDivElement | null>(null);
  const editButtonRef = useRef<HTMLButtonElement | null>(null);
  const editShellRef = useRef<HTMLDivElement | null>(null);
  const editorRef = useRef<HTMLDivElement | null>(null);
  const contextPills = restorePromptPillsFromMessage(message);
  const contextLoadStatus = (message.contextLoadStatus ?? []).filter((item) => item.loaded);
  const hasContext = Boolean(contextPills.length || contextLoadStatus.length);
  const isUser = message.role === "user";

  useEffect(() => {
    setDraftText(message.text);
  }, [message.text]);

  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(false), 1200);
    return () => window.clearTimeout(timer);
  }, [copied]);

  const saveEdit = () => {
    const nextText = draftText.trim();
    if (!nextText) {
      setDraftText(message.text);
      setEditing(false);
      return;
    }
    onUpdateMessageText(message, nextText);
    setEditing(false);
  };

  const cancelEdit = () => {
    setDraftText(message.text);
    setEditing(false);
  };

  useEffect(() => {
    if (!editing) return;
    const editor = editorRef.current;
    if (!editor) return;
    editor.textContent = draftText;
    editor.focus();
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(editor);
    range.collapse(false);
    selection?.removeAllRanges();
    selection?.addRange(range);
  }, [editing]);

  useEffect(() => {
    if (!contextOpen && !editing) return;
    const closeFloatingState = (event: PointerEvent) => {
      const target = event.target instanceof Node ? event.target : null;
      if (!target) return;
      if (
        contextOpen
        && !contextButtonRef.current?.contains(target)
        && !contextPopoverRef.current?.contains(target)
      ) {
        setContextOpen(false);
      }
      if (
        editing
        && !editShellRef.current?.contains(target)
        && !editButtonRef.current?.contains(target)
      ) {
        saveEdit();
      }
    };
    window.addEventListener("pointerdown", closeFloatingState);
    return () => window.removeEventListener("pointerdown", closeFloatingState);
  }, [contextOpen, editing, draftText]);

  const copyMessage = () => {
    onCopy(message.text);
    setCopied(true);
  };

  return (
    <article className={`chat-message ${message.role}${editing ? " editing" : ""}`}>
      <div className={`chat-message-content${editing ? " editing" : ""}`}>
        {editing ? (
          <div className="chat-message-edit-shell" ref={editShellRef}>
            <button
              type="button"
              className="chat-message-edit-cancel"
              onClick={cancelEdit}
              title="取消修改"
              aria-label="取消修改"
            >
              <MessageEditCancelIcon />
            </button>
            <div
              className="chat-message-editor"
              contentEditable
              ref={editorRef}
              suppressContentEditableWarning
              onInput={(event) => setDraftText(event.currentTarget.textContent ?? "")}
              onKeyDown={(event) => {
                if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
                  event.preventDefault();
                  saveEdit();
                }
                if (event.key === "Escape") {
                  event.preventDefault();
                  cancelEdit();
                }
              }}
            >
              {draftText}
            </div>
            <button
              type="button"
              className="chat-message-edit-submit"
              onClick={saveEdit}
              title="提交修改"
              aria-label="提交修改"
            >
              <MessageEditSubmitIcon />
            </button>
          </div>
        ) : (
          <div className="chat-message-bubble">
            <div className="chat-message-body">{message.text}</div>
          </div>
        )}
        {!editing ? (
          <div className="message-actions" aria-label="消息操作">
            {isUser ? (
              <span className="message-action-wrap">
                <button
                  type="button"
                  className={contextOpen ? "active" : ""}
                  ref={contextButtonRef}
                  onClick={() => setContextOpen((open) => !open)}
                  title="上下文"
                  aria-label="上下文"
                  disabled={!hasContext}
                >
                  <ContextIcon />
                </button>
                {contextOpen && hasContext ? (
                  <div className="message-context-popover" ref={contextPopoverRef}>
                    {contextPills.length ? (
                      <div className="message-context-row">
                        {contextPills.map((pill) => (
                          <span className={`message-context-pill ${pillClassName(pill)}`} key={pill.id}>
                            {pill.label}
                          </span>
                        ))}
                      </div>
                    ) : null}
                    {contextLoadStatus.length ? <ContextLoadStatusList status={contextLoadStatus} /> : null}
                  </div>
                ) : null}
              </span>
            ) : null}
            {isUser && canModify ? (
              <button
                type="button"
                ref={editButtonRef}
                onClick={() => setEditing(true)}
                title="修改"
                aria-label="修改"
              >
                <MessageEditIcon />
              </button>
            ) : (
              <button
                type="button"
                onClick={() => userForRetry ? onRetry(userForRetry) : undefined}
                disabled={!userForRetry}
                title="重试"
                aria-label="重试"
              >
                <RetryIcon />
              </button>
            )}
            <span className="message-copy-wrap">
              <button type="button" onClick={copyMessage} title="复制" aria-label="复制">
                <CopyIcon />
              </button>
              {copied ? <span className="message-copy-hint">复制成功</span> : null}
            </span>
          </div>
        ) : null}
      </div>
    </article>
  );
}

function ContextLoadStatusList({ status }: { status: PromptContextLoadStatus[] }) {
  const truncated = status.some((item) => item.truncated);
  return (
    <div className="message-context-status">
      {truncated ? <div className="message-context-note">部分内容已精简</div> : null}
      <ul>
        {status.map((item) => (
          <li key={item.key}>
            <span>{contextStatusLabel(item)}</span>
            <small>
              {contextStatusSummary(item)}
            </small>
            {item.truncated ? <em>内容较长，已保留关键部分。</em> : null}
          </li>
        ))}
      </ul>
    </div>
  );
}

function messageTime(message: ChatMessage) {
  return message.createdAt && Number.isFinite(message.createdAt) ? message.createdAt : Date.now();
}

function formatMessageTime(timestamp: number) {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    weekday: "short",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(timestamp));
}

function contextStatusLabel(item: PromptContextLoadStatus) {
  switch (item.key) {
    case "current-draft-selection":
      return "当前稿件";
    case "project-mode":
      return "项目记忆";
    case "active-context":
      return "最近对话现场";
    case "compressed-memory":
      return "压缩记忆";
    case "explicit-knowledge-cards":
      return "已选知识卡";
    case "relevant-notes":
      return "相关稿件";
    case "skill-protocol":
      return "技能规则";
    case "user-request":
      return "本次请求";
    default:
      return item.label || "上下文";
  }
}

function contextStatusSummary(item: PromptContextLoadStatus) {
  if (item.key === "current-draft-selection") {
    return item.itemCount > 1 ? "包含稿件和选区" : "包含当前稿件";
  }
  if (item.key === "user-request") {
    return "已读取你的输入";
  }
  if (item.itemCount > 1) {
    return `已读取 ${item.itemCount} 项`;
  }
  return "已读取";
}

function pillClassName(pill: PromptContextPill) {
  return `pill-${pill.kind}`;
}

function pillKindLabel(pill: PromptContextPill) {
  switch (pill.kind) {
    case "active-file":
      return "稿件";
    case "file":
      return "文件";
    case "image":
      return "图片";
    case "memory":
      return "知识";
    case "tool":
      return "技能";
    case "url":
      return "链接";
    case "selection":
      return "选区";
  }
}
