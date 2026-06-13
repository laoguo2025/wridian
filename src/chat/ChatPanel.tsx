import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { WridianPromptEditor } from "./WridianPromptEditor";
import {
  findPreviousUserMessage,
  restorePromptPillsFromMessage,
  type ChatMessage,
} from "./messageRepository";
import type { PromptContextLoadStatus, PromptContextPill, PromptSuggestion } from "./promptContext";
import type { ProjectConfig } from "./projectContext";
import type { ConfiguredModelStatus, RelevantNote } from "../appTypes";
import {
  ContextIcon,
  CopyIcon,
  MessageEditCancelIcon,
  MessageEditIcon,
  MessageEditSubmitIcon,
  RetryIcon,
} from "../icons";
import type { CoCreateFileOperation } from "./cocreationClient";

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
  onAddRelevantNote,
  onOpenRelevantNote,
  onRemovePill,
  onRetry,
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
  relevantNotesLoading,
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
  onAddRelevantNote: (note: RelevantNote) => void;
  onOpenRelevantNote: (note: RelevantNote) => void;
  onRemovePill: (id: string) => void;
  onRetry: (message: ChatMessage) => void;
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
  relevantNotesLoading: boolean;
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
      <div className="chat-thread" ref={threadRef}>
        {projectError ? <div className="chat-status error">{projectError}</div> : null}
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

      <RelevantNotesStrip
        error={relevantNotesError}
        loading={relevantNotesLoading}
        notes={relevantNotes}
        onAdd={onAddRelevantNote}
        onOpen={onOpenRelevantNote}
      />

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

function RelevantNotesStrip({
  error,
  loading,
  notes,
  onAdd,
  onOpen,
}: {
  error: string;
  loading: boolean;
  notes: RelevantNote[];
  onAdd: (note: RelevantNote) => void;
  onOpen: (note: RelevantNote) => void;
}) {
  const [open, setOpen] = useState(false);
  const knowledgeNotes = notes.filter((note) => note.kind === "knowledge");
  const draftNotes = notes.filter((note) => note.kind !== "knowledge");
  const hasContent = notes.length > 0 || loading || Boolean(error);

  useEffect(() => {
    if (notes.length) setOpen(true);
  }, [notes.length]);

  if (!hasContent) return null;

  return (
    <section className="relevant-notes" aria-label="相关内容">
      <button
        type="button"
        className="relevant-notes-trigger"
        aria-expanded={open}
        onClick={() => setOpen((current) => !current)}
      >
        <span>相关内容</span>
        <small>{loading ? "检索中" : notes.length ? `${notes.length} 项` : "无结果"}</small>
      </button>
      {open ? (
        <div className="relevant-notes-body">
          {error ? <div className="relevant-notes-error">{error}</div> : null}
          {knowledgeNotes.length ? (
            <RelevantNoteGroup label="知识" notes={knowledgeNotes} onAdd={onAdd} onOpen={onOpen} />
          ) : null}
          {draftNotes.length ? (
            <RelevantNoteGroup label="作品" notes={draftNotes} onAdd={onAdd} onOpen={onOpen} />
          ) : null}
          {!loading && !error && !notes.length ? <div className="relevant-notes-empty">暂无相关内容</div> : null}
        </div>
      ) : null}
    </section>
  );
}

function RelevantNoteGroup({
  label,
  notes,
  onAdd,
  onOpen,
}: {
  label: string;
  notes: RelevantNote[];
  onAdd: (note: RelevantNote) => void;
  onOpen: (note: RelevantNote) => void;
}) {
  return (
    <div className="relevant-note-group">
      <div className="relevant-note-group-label">{label}</div>
      <div className="relevant-note-list">
        {notes.map((note) => (
          <div className="relevant-note-row" key={`${note.kind}:${note.path}`}>
            <button
              type="button"
              className="relevant-note-main"
              onClick={() => onOpen(note)}
              title={note.relativePath ?? note.path}
            >
              <span>{note.title || note.relativePath || "未命名"}</span>
              <small>{relevantNoteDetail(note)}</small>
            </button>
            <button type="button" className="relevant-note-add" onClick={() => onAdd(note)}>
              加入
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}

function relevantNoteDetail(note: RelevantNote) {
  const reason = note.reasons.slice(0, 2).join(" · ");
  const fallback = note.snippet.trim().replace(/\s+/g, " ").slice(0, 56);
  return reason || fallback || note.relativePath || "相关文件";
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
            <div className={`chat-message-body ${isUser ? "plain" : "markdown"}`}>
              {isUser ? message.text : <MarkdownMessage text={message.text} />}
            </div>
            {message.fileOperations?.length ? <FileOperationBlocks operations={message.fileOperations} /> : null}
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
                    <div className="message-context-popover-head">
                      本轮发送时读取的上下文快照，只用于生成回复，不会自动改正文。
                    </div>
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

function MarkdownMessage({ text }: { text: string }) {
  return <div className="chat-markdown">{renderMarkdownBlocks(text)}</div>;
}

function renderMarkdownBlocks(text: string) {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }

    const codeFence = line.match(/^\s*```(\w+)?\s*$/);
    if (codeFence) {
      const language = codeFence[1];
      const codeLines: string[] = [];
      index += 1;
      while (index < lines.length && !/^\s*```\s*$/.test(lines[index])) {
        codeLines.push(lines[index]);
        index += 1;
      }
      if (index < lines.length) index += 1;
      blocks.push(
        <pre className="chat-markdown-codeblock" key={`code-${index}`}>
          <code data-language={language || undefined}>{codeLines.join("\n")}</code>
        </pre>,
      );
      continue;
    }

    const table = parseMarkdownTable(lines, index);
    if (table) {
      blocks.push(
        <div className="chat-markdown-table-wrap" key={`table-${index}`}>
          <table>
            <thead>
              <tr>
                {table.headers.map((header, cellIndex) => (
                  <th key={`h-${cellIndex}`}>{renderInlineMarkdown(header)}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {table.rows.map((row, rowIndex) => (
                <tr key={`r-${rowIndex}`}>
                  {table.headers.map((_, cellIndex) => (
                    <td key={`c-${cellIndex}`}>{renderInlineMarkdown(row[cellIndex] ?? "")}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>,
      );
      index = table.nextIndex;
      continue;
    }

    const heading = line.match(/^(#{1,4})\s+(.+)$/);
    if (heading) {
      const level = Math.min(heading[1].length, 4);
      const HeadingTag = `h${level}` as "h1" | "h2" | "h3" | "h4";
      blocks.push(<HeadingTag key={`heading-${index}`}>{renderInlineMarkdown(heading[2].trim())}</HeadingTag>);
      index += 1;
      continue;
    }

    if (/^\s*[-*]\s+/.test(line)) {
      const items: ReactNode[] = [];
      while (index < lines.length) {
        const match = lines[index].match(/^\s*[-*]\s+(.+)$/);
        if (!match) break;
        items.push(<li key={`ul-${index}`}>{renderInlineMarkdown(match[1].trim())}</li>);
        index += 1;
      }
      blocks.push(<ul key={`ul-block-${index}`}>{items}</ul>);
      continue;
    }

    if (/^\s*\d+[.)]\s+/.test(line)) {
      const items: ReactNode[] = [];
      while (index < lines.length) {
        const match = lines[index].match(/^\s*\d+[.)]\s+(.+)$/);
        if (!match) break;
        items.push(<li key={`ol-${index}`}>{renderInlineMarkdown(match[1].trim())}</li>);
        index += 1;
      }
      blocks.push(<ol key={`ol-block-${index}`}>{items}</ol>);
      continue;
    }

    const paragraphLines = [line.trim()];
    index += 1;
    while (
      index < lines.length
      && lines[index].trim()
      && !isMarkdownBlockStart(lines, index)
    ) {
      paragraphLines.push(lines[index].trim());
      index += 1;
    }
    blocks.push(<p key={`p-${index}`}>{renderInlineMarkdown(paragraphLines.join(" "))}</p>);
  }

  return blocks;
}

function isMarkdownBlockStart(lines: string[], index: number) {
  const line = lines[index];
  return Boolean(
    /^\s*```/.test(line)
    || /^(#{1,4})\s+/.test(line)
    || /^\s*[-*]\s+/.test(line)
    || /^\s*\d+[.)]\s+/.test(line)
    || parseMarkdownTable(lines, index),
  );
}

function parseMarkdownTable(lines: string[], startIndex: number) {
  const headerLine = lines[startIndex];
  const separatorLine = lines[startIndex + 1];
  if (!isMarkdownTableRow(headerLine) || !isMarkdownTableSeparator(separatorLine)) {
    return null;
  }

  const headers = splitMarkdownTableRow(headerLine);
  const rows: string[][] = [];
  let index = startIndex + 2;
  while (index < lines.length && isMarkdownTableRow(lines[index])) {
    rows.push(splitMarkdownTableRow(lines[index]));
    index += 1;
  }

  return { headers, rows, nextIndex: index };
}

function isMarkdownTableRow(line?: string) {
  return Boolean(line && line.includes("|") && splitMarkdownTableRow(line).length > 1);
}

function isMarkdownTableSeparator(line?: string) {
  if (!line || !line.includes("|")) return false;
  const cells = splitMarkdownTableRow(line);
  return cells.length > 1 && cells.every((cell) => /^:?-{3,}:?$/.test(cell.trim()));
}

function splitMarkdownTableRow(line: string) {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function renderInlineMarkdown(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\(https?:\/\/[^)\s]+\))/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text))) {
    if (match.index > lastIndex) nodes.push(text.slice(lastIndex, match.index));
    const token = match[0];
    const key = `${match.index}-${token}`;
    if (token.startsWith("`")) {
      nodes.push(<code key={key}>{token.slice(1, -1)}</code>);
    } else if (token.startsWith("**")) {
      nodes.push(<strong key={key}>{renderInlineMarkdown(token.slice(2, -2))}</strong>);
    } else {
      const link = token.match(/^\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)$/);
      if (link) {
        nodes.push(
          <a href={link[2]} key={key} rel="noreferrer" target="_blank">
            {link[1]}
          </a>,
        );
      } else {
        nodes.push(token);
      }
    }
    lastIndex = pattern.lastIndex;
  }

  if (lastIndex < text.length) nodes.push(text.slice(lastIndex));
  return nodes;
}

function FileOperationBlocks({ operations }: { operations: CoCreateFileOperation[] }) {
  return (
    <div className="chat-tool-results" aria-label="文件操作结果">
      {operations.map((operation, index) => (
        <div
          className={`chat-tool-result ${operation.ok ? "ok" : "error"}`}
          key={`${operation.action}:${operation.library}:${operation.path}:${index}`}
        >
          <div className="chat-tool-result-head">
            <span className="chat-tool-result-name">{fileOperationLabel(operation)}</span>
            <span className="chat-tool-result-status">{operation.ok ? "已执行" : "未执行"}</span>
          </div>
          <div className="chat-tool-result-path" title={operation.path}>
            {libraryLabel(operation.library)} / {operation.path || "未指定路径"}
          </div>
          <div className="chat-tool-result-message">{operation.message}</div>
        </div>
      ))}
    </div>
  );
}

function fileOperationLabel(operation: CoCreateFileOperation) {
  switch (operation.action) {
    case "writeFile":
      return "写入文件";
    case "createFolder":
      return "创建文件夹";
    case "rename":
      return "重命名";
    case "trash":
      return "移到回收站";
    default:
      return operation.action || "文件操作";
  }
}

function libraryLabel(library: string) {
  if (library === "knowledge") return "知识库";
  if (library === "works") return "作品库";
  return library || "文件库";
}

function ContextLoadStatusList({ status }: { status: PromptContextLoadStatus[] }) {
  const truncated = status.some((item) => item.truncated);
  return (
    <div className="message-context-status">
      {truncated ? <div className="message-context-note">部分内容较长，已保留关键片段。</div> : null}
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
    case "rule-router":
      return "规则路由";
    case "active-context":
      return "最近对话现场";
    case "compressed-memory":
      return "压缩记忆";
    case "explicit-knowledge-cards":
      return "已选知识卡";
    case "mentioned-files":
      return "点名文件";
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
    return item.itemCount > 1 ? "读取了当前稿件和选区" : "读取了当前稿件";
  }
  if (item.key === "project-mode") {
    return item.itemCount > 1 ? `读取了 ${item.itemCount} 项作品记忆` : "读取了作品记忆";
  }
  if (item.key === "explicit-knowledge-cards") {
    return item.itemCount > 1 ? `读取了 ${item.itemCount} 张知识卡` : "读取了已选知识卡";
  }
  if (item.key === "mentioned-files") {
    return item.itemCount > 1 ? `读取了 ${item.itemCount} 个点名文件` : "读取了点名文件";
  }
  if (item.key === "skill-protocol") {
    return item.itemCount > 1 ? `带入了 ${item.itemCount} 条技能规则` : "带入了技能规则";
  }
  if (item.key === "user-request") {
    return "已读取你的输入";
  }
  if (item.itemCount > 1) {
    return `读取了 ${item.itemCount} 项`;
  }
  return "已读取这一项";
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
