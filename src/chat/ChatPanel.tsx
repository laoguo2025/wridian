import { useEffect, useRef } from "react";
import { CopilotPromptEditor } from "./CopilotPromptEditor";
import {
  findPreviousUserMessage,
  restorePromptPillsFromMessage,
  type ChatMessage,
} from "./messageRepository";
import type { PromptContextPill, PromptSuggestion } from "./promptContext";
import type { ProjectConfig, RelevantNote } from "./projectContext";

export function ChatPanel({
  activeModelLabel,
  error,
  messages,
  onAddRelevantNote,
  onCopy,
  onEditUserMessage,
  onPromptChange,
  onPromptPillsChange,
  onImagePaste,
  onRemovePill,
  onRetry,
  onSelectSuggestion,
  onSelectProject,
  onSubmit,
  pending,
  projectError,
  projects,
  prompt,
  promptPills,
  promptSuggestions,
  relevantNotes,
  selectedProjectId,
}: {
  activeModelLabel: string;
  error: string;
  messages: ChatMessage[];
  onAddRelevantNote: (note: RelevantNote) => void;
  onCopy: (text: string) => void;
  onEditUserMessage: (message: ChatMessage) => void;
  onPromptChange: (value: string) => void;
  onPromptPillsChange: (pills: PromptContextPill[]) => void;
  onImagePaste: (files: File[]) => void;
  onRemovePill: (id: string) => void;
  onRetry: (message: ChatMessage) => void;
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  onSelectProject: (id: string) => void;
  onSubmit: () => void;
  pending: boolean;
  projectError: string;
  projects: ProjectConfig[];
  prompt: string;
  promptPills: PromptContextPill[];
  promptSuggestions: PromptSuggestion[];
  relevantNotes: RelevantNote[];
  selectedProjectId: string;
}) {
  const threadRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const thread = threadRef.current;
    if (!thread) return;
    thread.scrollTop = thread.scrollHeight;
  }, [error, messages.length, pending]);

  return (
    <aside className="chat-panel" aria-label="对话区">
      <div className="chat-modebar">
        <select value={selectedProjectId} onChange={(event) => onSelectProject(event.currentTarget.value)} aria-label="Project Mode">
          <option value="">普通聊天</option>
          {projects.map((project) => (
            <option value={project.id} key={project.id}>{project.name}</option>
          ))}
        </select>
      </div>
      {projectError ? <div className="chat-status error">{projectError}</div> : null}
      <div className="chat-thread" ref={threadRef}>
        {messages.length
          ? messages.map((message, index) => (
              <ChatMessageView
                key={message.id}
                message={message}
                onCopy={onCopy}
                onEditUserMessage={onEditUserMessage}
                onRetry={onRetry}
                userForRetry={findPreviousUserMessage(messages, index)}
              />
            ))
          : null}
        {pending ? <div className="chat-status">正在回复。</div> : null}
        {error ? <div className="chat-status error">{error}</div> : null}
      </div>

      {relevantNotes.length ? (
        <div className="relevant-notes" aria-label="Relevant Notes">
          {relevantNotes.map((note) => (
            <button type="button" key={note.path} onClick={() => onAddRelevantNote(note)} title={note.path}>
              <span>{note.title}</span>
              <small>{note.hasBacklinks ? "backlink" : note.hasOutgoingLinks ? "link" : note.score.toFixed(2)}</small>
            </button>
          ))}
        </div>
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
        <CopilotPromptEditor
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
          <select className="prompt-model-select" value={activeModelLabel || "未配置模型"} onChange={() => undefined} aria-label="模型选择">
            <option value={activeModelLabel || "未配置模型"}>{activeModelLabel || "未配置模型"}</option>
          </select>
          <button type="submit" className="prompt-send" aria-label={pending ? "停止" : "发送"} disabled={pending || !prompt.trim()}>
            {pending ? "..." : "发送"}
          </button>
        </div>
      </form>
    </aside>
  );
}

function ChatMessageView({
  message,
  onCopy,
  onEditUserMessage,
  onRetry,
  userForRetry,
}: {
  message: ChatMessage;
  onCopy: (text: string) => void;
  onEditUserMessage: (message: ChatMessage) => void;
  onRetry: (message: ChatMessage) => void;
  userForRetry?: ChatMessage;
}) {
  const contextPills = restorePromptPillsFromMessage(message);

  return (
    <article className={`chat-message ${message.role}`}>
      {message.selectedText ? (
        <div className="message-context-row">
          {contextPills.map((pill) => (
            <span className={`message-context-pill ${pillClassName(pill)}`} key={pill.id}>
              {pill.label}
            </span>
          ))}
        </div>
      ) : null}
      <div className="chat-message-body">{message.text}</div>
      <div className="message-actions">
        {message.role === "user" ? (
          <>
            <button type="button" onClick={() => onEditUserMessage(message)} title="编辑">
              编辑
            </button>
            <button type="button" onClick={() => onCopy(message.text)} title="复制">
              复制
            </button>
          </>
        ) : (
          <>
            <button type="button" onClick={() => userForRetry ? onRetry(userForRetry) : undefined} disabled={!userForRetry} title="重试">
              重试
            </button>
            <button type="button" onClick={() => onCopy(message.text)} title="复制">
              复制
            </button>
          </>
        )}
      </div>
    </article>
  );
}

function pillClassName(pill: PromptContextPill) {
  return `pill-${pill.kind}`;
}

function pillKindLabel(pill: PromptContextPill) {
  switch (pill.kind) {
    case "active-file":
      return "FILE";
    case "file":
      return "NOTE";
    case "image":
      return "IMG";
    case "memory":
      return "MEM";
    case "tool":
      return "TOOL";
    case "url":
      return "URL";
    case "selection":
      return "SEL";
  }
}
