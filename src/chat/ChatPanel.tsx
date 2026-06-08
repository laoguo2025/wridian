import { useEffect, useRef } from "react";
import { CopilotPromptEditor } from "./CopilotPromptEditor";
import {
  findPreviousUserMessage,
  restorePromptPillsFromMessage,
  type ChatMessage,
  type PromptContextPill,
  type PromptSuggestion,
} from "./messageRepository";

export function ChatPanel({
  error,
  messages,
  onAddToMemory,
  onCopy,
  onEditUserMessage,
  onPromptChange,
  onRemovePill,
  onRetry,
  onSelectSuggestion,
  onSubmit,
  pending,
  prompt,
  promptPills,
  promptSuggestions,
}: {
  error: string;
  messages: ChatMessage[];
  onAddToMemory: (text: string) => void;
  onCopy: (text: string) => void;
  onEditUserMessage: (message: ChatMessage) => void;
  onPromptChange: (value: string) => void;
  onRemovePill: (id: string) => void;
  onRetry: (message: ChatMessage) => void;
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  onSubmit: () => void;
  pending: boolean;
  prompt: string;
  promptPills: PromptContextPill[];
  promptSuggestions: PromptSuggestion[];
}) {
  const threadRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const thread = threadRef.current;
    if (!thread) return;
    thread.scrollTop = thread.scrollHeight;
  }, [error, messages.length, pending]);

  return (
    <aside className="chat-panel" aria-label="对话区">
      <div className="chat-thread" ref={threadRef}>
        {messages.length
          ? messages.map((message, index) => (
              <ChatMessageView
                key={message.id}
                message={message}
                onAddToMemory={onAddToMemory}
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
              <span className="prompt-attachment" key={pill.id}>
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
          onSelectSuggestion={onSelectSuggestion}
          onSubmit={onSubmit}
          placeholder="与 Wridian 对话"
          suggestions={promptSuggestions}
        />
        <button type="submit" className="prompt-send" aria-label={pending ? "停止" : "发送"} disabled={pending || !prompt.trim()}>
          {pending ? "..." : "↵"}
        </button>
      </form>
    </aside>
  );
}

function ChatMessageView({
  message,
  onAddToMemory,
  onCopy,
  onEditUserMessage,
  onRetry,
  userForRetry,
}: {
  message: ChatMessage;
  onAddToMemory: (text: string) => void;
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
            <span className="message-context-pill" key={pill.id}>
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
            <button type="button" onClick={() => onAddToMemory(message.text)} title="添加到记忆">
              记忆
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
            <button type="button" onClick={() => onAddToMemory(message.text)} title="添加到记忆">
              记忆
            </button>
          </>
        )}
      </div>
    </article>
  );
}
