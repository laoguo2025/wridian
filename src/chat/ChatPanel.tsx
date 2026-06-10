import { useEffect, useRef } from "react";
import { CopilotPromptEditor } from "./CopilotPromptEditor";
import {
  findPreviousUserMessage,
  restorePromptPillsFromMessage,
  type ChatMessage,
} from "./messageRepository";
import type { PromptContextPill, PromptSuggestion } from "./promptContext";
import type { ProjectConfig } from "./projectContext";
import type { ConfiguredModelStatus } from "../appTypes";

export function ChatPanel({
  activeModelLabel,
  configuredModels,
  error,
  messages,
  onCopy,
  onEditUserMessage,
  onPromptChange,
  onPromptPillsChange,
  onImagePaste,
  onRemovePill,
  onRetry,
  onSelectModel,
  onSelectSuggestion,
  onSelectProject,
  onSubmit,
  pending,
  projectError,
  projects,
  prompt,
  promptPills,
  promptSuggestions,
  selectedProjectId,
  selectedModelId,
}: {
  activeModelLabel: string;
  configuredModels: ConfiguredModelStatus[];
  error: string;
  messages: ChatMessage[];
  onCopy: (text: string) => void;
  onEditUserMessage: (message: ChatMessage) => void;
  onPromptChange: (value: string) => void;
  onPromptPillsChange: (pills: PromptContextPill[]) => void;
  onImagePaste: (files: File[]) => void;
  onRemovePill: (id: string) => void;
  onRetry: (message: ChatMessage) => void;
  onSelectModel: (id: string) => void;
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  onSelectProject: (id: string) => void;
  onSubmit: () => void;
  pending: boolean;
  projectError: string;
  projects: ProjectConfig[];
  prompt: string;
  promptPills: PromptContextPill[];
  promptSuggestions: PromptSuggestion[];
  selectedProjectId: string;
  selectedModelId: string;
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
          <button type="submit" className="prompt-send" aria-label={pending ? "停止" : "发送"} disabled={pending || (!prompt.trim() && !promptPills.length)}>
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
      {contextPills.length ? (
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
