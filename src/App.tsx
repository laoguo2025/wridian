import { KeyboardEvent as ReactKeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  createAssistantChatMessage,
  createUserChatMessage,
  restorePromptPillsFromMessage,
  type ChatMessage,
  type PromptContextPill,
  type PromptSuggestion,
} from "./chat/messageRepository";
import { ChatPanel } from "./chat/ChatPanel";
import {
  createDraftReplaceGuardReport,
  describeDraftReplaceSkip,
} from "./editor/draftReplaceGuard";
import "./App.css";

type Theme = "light" | "dark";
type SaveStatus = "idle" | "dirty" | "saving" | "saved" | "error";

type WorkspaceInfo = {
  vaultPath: string;
  runtimePath: string;
  filesRootPath: string;
  activeWorkRoot?: string | null;
  files: WorkFileNode[];
};

type WorkFileNode = {
  name: string;
  path: string;
  folder: boolean;
  children: WorkFileNode[];
};

type OpenFileResponse = {
  path: string;
  name: string;
  content: string;
};

type SaveFileResponse = {
  ok: boolean;
  savedAt: string;
};

type CustomApiSettingsStatus = {
  configured: boolean;
  baseUrl?: string | null;
  model?: string | null;
  maskedKey?: string | null;
};

type TestCustomApiResponse = {
  ok: boolean;
  message: string;
};

type CoCreateResponse = {
  reply: string;
  edits: CoCreateEdit[];
  memoriesUsed: string[];
};

type CoCreateEdit = {
  target: string;
  replacement: string;
  rationale?: string | null;
};

type DraftEdit = CoCreateEdit & {
  id: string;
  status: "pending" | "accepted" | "rejected";
};

type MemoryItem = {
  id: string;
  category?: string;
  text: string;
  sourcePath: string;
  title: string;
  createdAt: string;
};

type MemoryCandidate = {
  id: string;
  category?: string;
  text: string;
  sourcePath: string;
  title: string;
  createdAt: string;
};

type MemoryState = {
  memories: MemoryItem[];
  candidates: MemoryCandidate[];
  memoryFolderPath?: string;
};

type FileContextMenu = {
  node: WorkFileNode;
  x: number;
  y: number;
};

type TextSelection = {
  start: number;
  end: number;
};

function App() {
  const [theme, setTheme] = useState<Theme>("light");
  const [memoryOpen, setMemoryOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [workspaceError, setWorkspaceError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [cocreating, setCocreating] = useState(false);
  const [cocreationError, setCocreationError] = useState("");
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [pendingEdits, setPendingEdits] = useState<DraftEdit[]>([]);
  const [promptPills, setPromptPills] = useState<PromptContextPill[]>([]);
  const [hasDraftSelection, setHasDraftSelection] = useState(false);
  const [draftSelection, setDraftSelection] = useState<TextSelection>({ start: 0, end: 0 });
  const [selectedPath, setSelectedPath] = useState("");
  const [editorTitle, setEditorTitle] = useState("");
  const [editorContent, setEditorContent] = useState("");
  const [lastSavedContent, setLastSavedContent] = useState("");
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState("");
  const [memoryState, setMemoryState] = useState<MemoryState>({ memories: [], candidates: [] });
  const [memoryError, setMemoryError] = useState("");
  const [extractingMemory, setExtractingMemory] = useState(false);
  const [fileMenu, setFileMenu] = useState<FileContextMenu | null>(null);
  const draftEditorRef = useRef<HTMLDivElement | null>(null);
  const draftSelectionRef = useRef<TextSelection>({ start: editorContent.length, end: editorContent.length });

  const loadMemoryState = useCallback(async () => {
    try {
      const response = await invoke<MemoryState>("wridian_get_memory_state");
      setMemoryState(response.memories.length || response.candidates.length ? response : { memories: [], candidates: [] });
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const sendPrompt = async (override?: { text: string; selectedText?: string }) => {
    const userInput = (override?.text ?? prompt).trim();
    if (!userInput || cocreating) return;
    if (!override) setPrompt("");
    setMemoryOpen(false);
    setCocreating(true);
    setCocreationError("");
    const userMessage = createUserChatMessage({
      contextPills: override ? [] : promptPills,
      selectedText: override?.selectedText,
      text: userInput,
    });
    const selectedText = userMessage.selectedText ?? "";
    setChatMessages((messages) => [...messages, userMessage]);
    if (!override) setPromptPills([]);
    try {
      const response = await invoke<CoCreateResponse>("wridian_cocreate", {
        input: {
          sourcePath: selectedPath || "未选择文件",
          title: editorTitle || "未选择文件",
          content: editorContent,
          userInput,
          selectedText: selectedText || null,
        },
      });
      setChatMessages((messages) => [...messages, createAssistantChatMessage(response.reply)]);
      setPendingEdits((edits) => [
        ...edits,
        ...response.edits.map((edit, index) => ({ ...edit, id: `edit-${Date.now()}-${index}`, status: "pending" as const })),
      ]);
    } catch (error) {
      setCocreationError(error instanceof Error ? error.message : String(error));
    } finally {
      setCocreating(false);
    }
  };

  const updateDraftSelection = useCallback(() => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const { start, end } = selection;
    draftSelectionRef.current = { start, end };
    setDraftSelection({ start, end });
    setHasDraftSelection(end > start);
  }, []);

  const attachCurrentSelectionToPrompt = () => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const selected = editorContent.slice(selection.start, selection.end).trim();
    if (!selected) return;
    setPromptPills((current) => [
      ...current.filter((pill) => pill.id !== "selection"),
      { id: "selection", label: "选区", value: selected },
    ]);
    setPrompt((current) => current || "请修改这段。");
  };

  useEffect(() => {
    document.documentElement.classList.toggle("darkTheme", theme === "dark");
  }, [theme]);

  useEffect(() => {
    void invoke<WorkspaceInfo>("wridian_init_workspace")
      .then((response) => {
        setWorkspace(response);
        setWorkspaceError("");
      })
      .catch(() => setWorkspace(null));
  }, []);

  useEffect(() => {
    void loadMemoryState();
  }, [loadMemoryState]);

  const files = workspace?.files ?? [];
  const isRealFile = Boolean(selectedPath);
  const dirty = isRealFile && editorContent !== lastSavedContent;

  const saveCurrentFile = useCallback(async () => {
    if (!isRealFile || !dirty) return;
    setSaveStatus("saving");
    setSaveError("");
    try {
      await invoke<SaveFileResponse>("wridian_save_file", {
        input: { path: selectedPath, content: editorContent },
      });
      setLastSavedContent(editorContent);
      setSaveStatus("saved");
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  }, [dirty, editorContent, isRealFile, selectedPath]);

  useEffect(() => {
    if (!isRealFile) return;
    if (!dirty) {
      setSaveStatus("saved");
      return;
    }
    setSaveStatus("dirty");
    const timer = window.setTimeout(() => {
      void saveCurrentFile();
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [dirty, isRealFile, saveCurrentFile]);

  const openWorkFolder = async () => {
    setWorkspaceError("");
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected !== "string") return;
      const response = await invoke<WorkspaceInfo>("wridian_set_work_root", { input: { path: selected } });
      setWorkspace(response);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端选择本地文件夹。" : message);
    }
  };

  const refreshWorkspace = (response: WorkspaceInfo) => {
    setWorkspace(response);
    setWorkspaceError("");
  };

  const workspaceRootPath = workspace?.filesRootPath || workspace?.activeWorkRoot || workspace?.vaultPath || "";

  const runWorkspaceAction = async (action: () => Promise<WorkspaceInfo>) => {
    setWorkspaceError("");
    try {
      refreshWorkspace(await action());
    } catch (error) {
      setWorkspaceError(error instanceof Error ? error.message : String(error));
    }
  };

  const createFile = async (parentPath = workspaceRootPath) => {
    if (!parentPath) return;
    const name = window.prompt("新建文件", "未命名.md");
    if (!name) return;
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_create_work_file", { input: { parentPath, name } }));
  };

  const createFolder = async (parentPath = workspaceRootPath) => {
    if (!parentPath) return;
    const name = window.prompt("新建文件夹", "新建文件夹");
    if (!name) return;
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_create_work_folder", { input: { parentPath, name } }));
  };

  const duplicateNode = async (node: WorkFileNode) => {
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_duplicate_work_node", { input: { path: node.path } }));
  };

  const renameNode = async (node: WorkFileNode) => {
    const name = window.prompt("重命名", node.name);
    if (!name || name === node.name) return;
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_rename_work_node", { input: { path: node.path, newName: name } }));
  };

  const trashNode = async (node: WorkFileNode) => {
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_trash_work_node", { input: { path: node.path } }));
  };

  const addNodeToPrompt = (node: WorkFileNode) => {
    if (node.folder) return;
    setPromptPills((current) => [
      ...current.filter((pill) => pill.id !== `file:${node.path}`),
      { id: `file:${node.path}`, label: node.name, value: `路径：${node.path}` },
    ]);
  };

  const openFileContextMenu = (node: WorkFileNode, x: number, y: number) => {
    setFileMenu({ node, x, y });
  };

  useEffect(() => {
    if (!fileMenu) return;
    const close = () => setFileMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", close);
    };
  }, [fileMenu]);

  const openFile = async (node: WorkFileNode) => {
    if (node.folder) return;
    setSelectedPath(node.path);
    setEditorTitle(node.name);
    setSaveError("");
    setSaveStatus("idle");
    try {
      const response = await invoke<OpenFileResponse>("wridian_open_file", { input: { path: node.path } });
      setSelectedPath(response.path);
      setEditorTitle(response.name);
      setEditorContent(response.content);
      setLastSavedContent(response.content);
      draftSelectionRef.current = { start: response.content.length, end: response.content.length };
      setDraftSelection({ start: response.content.length, end: response.content.length });
      setHasDraftSelection(false);
      setPromptPills([]);
      setPendingEdits([]);
      setSaveStatus("saved");
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleDraftKeyDown = (event: ReactKeyboardEvent<HTMLElement>) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveCurrentFile();
    }
  };

  const applyTextToDraft = useCallback((text: string, selection: TextSelection) => {
    const start = Math.max(0, Math.min(selection.start, editorContent.length));
    const end = Math.max(start, Math.min(selection.end, editorContent.length));
    const nextContent = `${editorContent.slice(0, start)}${text}${editorContent.slice(end)}`;
    const nextCursor = start + text.length;
    setEditorContent(nextContent);
    draftSelectionRef.current = { start: nextCursor, end: nextCursor };
    setDraftSelection({ start: nextCursor, end: nextCursor });
    setHasDraftSelection(false);
    window.requestAnimationFrame(() => {
      setContentEditableCaret(draftEditorRef.current, nextCursor);
    });
  }, [editorContent]);

  const copyText = async (text: string) => {
    const reply = text.trim();
    if (!reply) return;
    try {
      await navigator.clipboard.writeText(reply);
    } catch {
    }
  };

  const addTextToMemory = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    try {
      const response = await invoke<MemoryState>("wridian_create_memory_candidate", {
        input: {
          sourcePath: selectedPath,
          title: editorTitle,
          content: editorContent,
          userIntent: trimmed,
        },
      });
      setMemoryState(response);
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  const editUserMessage = (message: ChatMessage) => {
    setPrompt(message.text);
    setPromptPills(restorePromptPillsFromMessage(message));
  };

  const retryLastUserMessage = (message: ChatMessage) => {
    void sendPrompt({ text: message.text, selectedText: message.selectedText });
  };

  const acceptEdit = (id: string) => {
    const edit = pendingEdits.find((item) => item.id === id && item.status === "pending");
    if (!edit) return;
    const guardReport = createDraftReplaceGuardReport(editorContent, pendingEdits.filter((item) => item.status === "pending"));
    const match = guardReport.matches.find((item) => item.edit.id === id);
    if (!match) {
      const skipped = guardReport.skipped.find((item) => item.edit.id === id);
      setCocreationError(skipped ? describeDraftReplaceSkip(skipped.reason) : "这处修改无法安全定位。");
      return;
    }
    setCocreationError("");
    applyTextToDraft(edit.replacement, { start: match.index, end: match.index + edit.target.length });
    setPendingEdits((edits) => edits.map((item) => (item.id === id ? { ...item, status: "accepted" } : item)));
  };

  const rejectEdit = (id: string) => {
    setPendingEdits((edits) => edits.map((item) => (item.id === id ? { ...item, status: "rejected" } : item)));
  };

  const acceptAllEdits = () => {
    const pending = pendingEdits.filter((edit) => edit.status === "pending");
    const guardReport = createDraftReplaceGuardReport(editorContent, pending);
    const matches = guardReport.matches;

    if (!matches.length) {
      setCocreationError("没有可以安全确认的修改。");
      return;
    }

    const appliedIds = new Set(matches.map((match) => match.edit.id));
    const nextContent = [...matches].sort((left, right) => right.index - left.index).reduce((content, match) => {
      const start = match.index;
      const end = start + match.edit.target.length;
      return `${content.slice(0, start)}${match.edit.replacement}${content.slice(end)}`;
    }, editorContent);

    setEditorContent(nextContent);
    setPendingEdits((edits) => edits.map((edit) => (appliedIds.has(edit.id) ? { ...edit, status: "accepted" } : edit)));
    setCocreationError(guardReport.skipped.length ? `${guardReport.skipped.length} 处修改需要重新定位。` : "");
    draftSelectionRef.current = { start: 0, end: 0 };
    setDraftSelection({ start: 0, end: 0 });
    setHasDraftSelection(false);

  };

  const rejectAllEdits = () => {
    setPendingEdits((edits) => edits.map((edit) => (edit.status === "pending" ? { ...edit, status: "rejected" } : edit)));
  };

  const pendingDraftEdits = useMemo(() => pendingEdits.filter((edit) => edit.status === "pending"), [pendingEdits]);
  const draftReplaceGuardReport = useMemo(
    () => createDraftReplaceGuardReport(editorContent, pendingDraftEdits),
    [editorContent, pendingDraftEdits],
  );
  const blockedDraftEditCount = draftReplaceGuardReport.skipped.length;
  const promptSuggestions = useMemo(() => {
    const suggestions: PromptSuggestion[] = [];
    const selectedDraftText = editorContent.slice(draftSelection.start, draftSelection.end).trim();

    if (selectedDraftText) {
      suggestions.push({
        id: "selection",
        label: "当前选区",
        detail: "把正文里划选的片段作为本轮上下文",
        insertText: selectedDraftText,
        kind: "context",
      });
    }

    if (selectedPath && editorContent.trim()) {
      suggestions.push(
        {
          id: "current-file",
          label: "当前文件",
          detail: editorTitle || baseName(selectedPath),
          insertText: `标题：${editorTitle || baseName(selectedPath)}\n\n${editorContent}`,
          kind: "context",
        },
        {
          id: "current-draft",
          label: "当前正文",
          detail: "把全文作为重点上下文",
          insertText: editorContent,
          kind: "context",
        },
      );
    }

    suggestions.push(
      {
        id: "rewrite-dialogue",
        label: "改对白",
        detail: "让对白更像角色本人、更适合小说或短剧表演",
        insertText: "请把这段对白改得更符合角色口吻，并增强短剧冲突。",
        kind: "command",
      },
      {
        id: "raise-conflict",
        label: "增强冲突",
        detail: "提高场景里的阻力、误会、压迫感或选择成本",
        insertText: "请增强这一段的戏剧冲突，但不要改变既有人物关系和事件顺序。",
        kind: "command",
      },
      {
        id: "add-hook",
        label: "加结尾钩子",
        detail: "补一个适合章节、分场或短剧结尾的悬念",
        insertText: "请给这一段补一个结尾钩子，让读者或观众想继续看下一段。",
        kind: "command",
      },
      {
        id: "voice-check",
        label: "检查角色口吻",
        detail: "检查人物说话是否串味，指出并改写",
        insertText: "请检查这一段的角色口吻是否一致，指出问题并给出修改建议。",
        kind: "command",
      },
      {
        id: "rename-character",
        label: "批量修改角色名",
        detail: "跨段落替换当前文件里的角色名",
        insertText: "请把当前文件里的角色名从「旧名字」批量改成「新名字」，并保持上下文自然。",
        kind: "command",
      },
      {
        id: "extract-memory",
        label: "提取记忆",
        detail: "提取人物、设定、伏笔、风格、禁区和剧本规则",
        insertText: "请从当前稿件中提取可以进入写作记忆的人物、设定、伏笔、风格、禁区和剧本规则。",
        kind: "command",
      },
    );

    return suggestions;
  }, [draftSelection.end, draftSelection.start, editorContent, editorTitle, selectedPath]);

  const statusLabel = useMemo(() => {
    if (saveStatus === "idle") return "读取中";
    if (saveStatus === "dirty") return "未保存";
    if (saveStatus === "saving") return "正在保存";
    if (saveStatus === "error") return "保存失败";
    return "已保存";
  }, [saveStatus]);

  const acceptMemoryCandidate = async (id: string) => {
    try {
      const response = await invoke<MemoryState>("wridian_accept_memory_candidate", { input: { id } });
      setMemoryState(response);
      setMemoryError("");
    } catch (error) {
      setMemoryState((current) => {
        const candidate = current.candidates.find((item) => item.id === id);
        if (!candidate) return current;
        return {
          memories: [...current.memories, { ...candidate, id: `local-memory-${Date.now()}` }],
          candidates: current.candidates.filter((item) => item.id !== id),
        };
      });
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  const ignoreMemoryCandidate = async (id: string) => {
    try {
      const response = await invoke<MemoryState>("wridian_ignore_memory_candidate", { input: { id } });
      setMemoryState(response);
      setMemoryError("");
    } catch (error) {
      setMemoryState((current) => ({
        ...current,
        candidates: current.candidates.filter((item) => item.id !== id),
      }));
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  const updateMemoryCandidate = async (id: string, text: string) => {
    try {
      const response = await invoke<MemoryState>("wridian_update_memory_candidate", { input: { id, text } });
      setMemoryState(response);
      setMemoryError("");
    } catch (error) {
      setMemoryState((current) => ({
        ...current,
        candidates: current.candidates.map((candidate) => (candidate.id === id ? { ...candidate, text } : candidate)),
      }));
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  const extractMemoryCandidates = async () => {
    setExtractingMemory(true);
    setMemoryError("");
    try {
      if (!("__TAURI_INTERNALS__" in window)) {
        throw new Error("请在 Wridian 桌面端使用正文记忆提取。");
      }
      const response = await invoke<MemoryState>("wridian_extract_memory_candidates", {
        input: {
          sourcePath: selectedPath,
          title: editorTitle,
          content: editorContent,
        },
      });
      setMemoryState(response);
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    } finally {
      setExtractingMemory(false);
    }
  };

  const openMemoryFolder = async () => {
    if (!memoryState.memoryFolderPath) {
      setMemoryError("请在 Wridian 桌面端打开记忆文件夹。");
      return;
    }
    try {
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(memoryState.memoryFolderPath);
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" />
          <span>Wridian</span>
        </div>
        <nav className="top-actions" aria-label="Wridian actions">
          <button type="button" onClick={() => {
            setMemoryOpen(true);
          }}>
            记忆
          </button>
          <button type="button" onClick={() => setSettingsOpen(true)}>
            模型
          </button>
          <button type="button" onClick={() => setTheme(theme === "light" ? "dark" : "light")}>
            {theme === "light" ? "深色" : "浅色"}
          </button>
        </nav>
      </header>

      <div className="workspace">
        <aside className="project-rail" aria-label="作品">
          <div className="rail-topline">
            <div className="rail-section-title">作品</div>
            <div className="file-toolbar" aria-label="文件操作">
              <button type="button" title="新建文件" aria-label="新建文件" onClick={() => void createFile()}>
                <PencilIcon />
              </button>
              <button type="button" title="新建文件夹" aria-label="新建文件夹" onClick={() => void createFolder()}>
                <FolderPlusIcon />
              </button>
              <button type="button" title="作品文件夹" aria-label="作品文件夹" onClick={() => void openWorkFolder()}>
                <WorkFolderIcon />
              </button>
            </div>
          </div>
          {workspaceError ? <div className="rail-error">{workspaceError}</div> : null}

          <div className="file-tree">
            {files.map((node) => (
              <FileNodeView
                key={node.path}
                node={node}
                depth={0}
                selectedPath={selectedPath}
                onOpenFile={openFile}
                onOpenMenu={openFileContextMenu}
              />
            ))}
          </div>

          <div className="rail-bottom">
            <button type="button" title="系统设置" aria-label="系统设置" onClick={() => setSettingsOpen(true)}>
              <SettingsIcon />
            </button>
          </div>
        </aside>

        <main className="writing-pane">
          <section className={`paper ${selectedPath ? "" : "paper-empty"}`} aria-label="正文编辑区">
            {selectedPath ? (
              <div className="paper-topline">
                <div className="paper-kicker">{baseName(selectedPath)}</div>
                <div className="paper-actions">
                  <button type="button" className="paper-action" onClick={attachCurrentSelectionToPrompt} disabled={!hasDraftSelection}>
                    添加选区到输入框
                  </button>
                  <button type="button" className="paper-action" onClick={() => {
                    setMemoryOpen(true);
                    void extractMemoryCandidates();
                  }} disabled={!editorContent.trim()}>
                    提取当前内容到记忆
                  </button>
                  <div className={`save-state ${saveStatus}`} title={saveError || undefined}>
                    {statusLabel}
                  </div>
                </div>
              </div>
            ) : null}
            {selectedPath ? (
              <>
                <h1 className="chapter-heading">{editorTitle}</h1>
                <div className="draft-suggestion-actions" aria-label="待确认修改操作" hidden={!pendingDraftEdits.length}>
                    <span>{pendingDraftEdits.length} 处待确认修改</span>
                    {blockedDraftEditCount ? <span className="draft-guard-note">{blockedDraftEditCount} 处需重新定位</span> : null}
                    <button type="button" onClick={acceptAllEdits}>全部确认</button>
                    <button type="button" className="secondary" onClick={rejectAllEdits}>全部取消</button>
                </div>
                <DraftEditor
                  content={editorContent}
                  edits={pendingDraftEdits}
                  editorRef={draftEditorRef}
                  onAcceptEdit={acceptEdit}
                  onChange={setEditorContent}
                  onKeyDown={handleDraftKeyDown}
                  onRejectEdit={rejectEdit}
                  onSelectionChange={updateDraftSelection}
                />
              </>
            ) : (
              <div className="empty-editor" aria-label="空文件编辑区">文件编辑区</div>
            )}
            {saveError ? <div className="paper-error">{saveError}</div> : null}
          </section>
        </main>

        <ChatPanel
          error={cocreationError}
          messages={chatMessages}
          onAddToMemory={addTextToMemory}
          onCopy={copyText}
          onEditUserMessage={editUserMessage}
          onRetry={retryLastUserMessage}
          pending={cocreating}
          prompt={prompt}
          promptPills={promptPills}
          promptSuggestions={promptSuggestions}
          onPromptChange={setPrompt}
          onRemovePill={(id) => setPromptPills((current) => current.filter((pill) => pill.id !== id))}
          onSelectSuggestion={(suggestion) => {
            if (suggestion.kind !== "context") return;
            setPromptPills((current) => [
              ...current.filter((pill) => pill.id !== suggestion.id),
              { id: suggestion.id, label: suggestion.label, value: suggestion.insertText },
            ]);
          }}
          onSubmit={() => void sendPrompt()}
        />
      </div>

      {memoryOpen ? (
        <MemoryDrawer
          currentTitle={editorTitle}
          memoryError={memoryError}
          memoryState={memoryState}
          onAcceptCandidate={acceptMemoryCandidate}
          onClose={() => setMemoryOpen(false)}
          onExtractCandidates={extractMemoryCandidates}
          onIgnoreCandidate={ignoreMemoryCandidate}
          onOpenMemoryFolder={openMemoryFolder}
          onUpdateCandidate={updateMemoryCandidate}
          extracting={extractingMemory}
          workspace={workspace}
        />
      ) : null}
      {settingsOpen ? <ModelSettingsDialog onClose={() => setSettingsOpen(false)} /> : null}
      {fileMenu ? (
        <FileContextMenuView
          menu={fileMenu}
          onAddToPrompt={addNodeToPrompt}
          onClose={() => setFileMenu(null)}
          onCreateFile={createFile}
          onCreateFolder={createFolder}
          onDuplicate={duplicateNode}
          onRename={renameNode}
          onTrash={trashNode}
        />
      ) : null}
    </div>
  );
}

function FileNodeView({
  depth,
  node,
  onOpenFile,
  onOpenMenu,
  selectedPath,
}: {
  depth: number;
  node: WorkFileNode;
  onOpenFile: (node: WorkFileNode) => void;
  onOpenMenu: (node: WorkFileNode, x: number, y: number) => void;
  selectedPath: string;
}) {
  const [expanded, setExpanded] = useState(true);
  const isFolder = node.folder;
  const hasChildren = isFolder && node.children.length > 0;
  const fileExt = isFolder ? "" : fileExtension(node.name);
  const rowClassName = [
    "file-row",
    isFolder ? "folder" : "file",
    node.path === selectedPath ? "active" : "",
    isFolder && expanded ? "expanded" : "",
    isFolder && !expanded ? "collapsed" : "",
    isFolder && !hasChildren ? "empty-folder" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const handleOpen = () => {
    if (isFolder) {
      setExpanded((current) => !current);
      return;
    }

    onOpenFile(node);
  };

  return (
    <div className="file-node">
      <button
        className={rowClassName}
        type="button"
        aria-expanded={isFolder ? expanded : undefined}
        title={node.path}
        onClick={handleOpen}
        onContextMenu={(event) => {
          event.preventDefault();
          onOpenMenu(node, event.clientX, event.clientY);
        }}
      >
        <span className="tree-toggle" aria-hidden="true" />
        <strong>{isFolder ? node.name : fileTitle(node.name)}</strong>
        {fileExt ? <span className="file-ext">{fileExt}</span> : null}
      </button>
      {hasChildren && expanded ? (
        <div className="file-children">
          {node.children.map((child) => (
            <FileNodeView
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onOpenFile={onOpenFile}
              onOpenMenu={onOpenMenu}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function fileTitle(name: string) {
  const extensionStart = name.lastIndexOf(".");
  if (extensionStart <= 0) return name;
  return name.slice(0, extensionStart);
}

function fileExtension(name: string) {
  const extensionStart = name.lastIndexOf(".");
  if (extensionStart <= 0 || extensionStart === name.length - 1) return "";
  return name.slice(extensionStart + 1);
}

function FileContextMenuView({
  menu,
  onAddToPrompt,
  onClose,
  onCreateFile,
  onCreateFolder,
  onDuplicate,
  onRename,
  onTrash,
}: {
  menu: FileContextMenu;
  onAddToPrompt: (node: WorkFileNode) => void;
  onClose: () => void;
  onCreateFile: (parentPath?: string) => Promise<void>;
  onCreateFolder: (parentPath?: string) => Promise<void>;
  onDuplicate: (node: WorkFileNode) => Promise<void>;
  onRename: (node: WorkFileNode) => Promise<void>;
  onTrash: (node: WorkFileNode) => Promise<void>;
}) {
  const run = (action: () => void | Promise<void>) => {
    onClose();
    void action();
  };

  return (
    <div className="context-menu" style={{ left: menu.x, top: menu.y }} onClick={(event) => event.stopPropagation()}>
      {menu.node.folder ? (
        <>
          <button type="button" onClick={() => run(() => onCreateFile(menu.node.path))}>
            新建文件
          </button>
          <button type="button" onClick={() => run(() => onCreateFolder(menu.node.path))}>
            新建文件夹
          </button>
        </>
      ) : null}
      <button type="button" onClick={() => run(() => onDuplicate(menu.node))}>
        创建副本
      </button>
      <button type="button" onClick={() => run(() => onAddToPrompt(menu.node))}>
        添加到共创输入
      </button>
      <button type="button" onClick={() => run(() => onRename(menu.node))}>
        重命名
      </button>
      <button type="button" className="danger" onClick={() => run(() => onTrash(menu.node))}>
        移到回收站
      </button>
    </div>
  );
}

function baseName(path: string) {
  return path.replace(/[\\/]+$/g, "").split(/[\\/]/).pop() || path;
}

function PencilIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M10 44H38C39.1046 44 40 43.1046 40 42V14H30V4H10C8.89543 4 8 4.89543 8 6V42C8 43.1046 8.89543 44 10 44Z" />
      <path d="M30 4L40 14" />
      <path d="M24 21V35" />
      <path d="M17 28H24L31 28" />
    </svg>
  );
}

function FolderPlusIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M5 8C5 6.89543 5.89543 6 7 6H19L24 12H41C42.1046 12 43 12.8954 43 14V40C43 41.1046 42.1046 42 41 42H7C5.89543 42 5 41.1046 5 40V8Z" />
      <path d="M18 27H30" />
      <path d="M24 21L24 33" />
    </svg>
  );
}

function WorkFolderIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M43 23V14C43 12.8954 42.1046 12 41 12H24L19 6H7C5.89543 6 5 6.89543 5 8V40C5 41.1046 5.89543 42 7 42H22" />
      <circle cx="35" cy="35" r="4" />
      <path d="M35 28V31" />
      <path d="M35 39V42" />
      <path d="M39.8281 30L37.7068 32.1213" />
      <path d="M31.8281 38L29.7068 40.1213" />
      <path d="M30 30L32.1213 32.1213" />
      <path d="M38 38L40.1213 40.1213" />
      <path d="M28 35H29.5H31" />
      <path d="M39 35H40.5H42" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M18.2838 43.1713C14.9327 42.1736 11.9498 40.3213 9.58787 37.867C10.469 36.8227 11 35.4734 11 34.0001C11 30.6864 8.31371 28.0001 5 28.0001C4.79955 28.0001 4.60139 28.01 4.40599 28.0292C4.13979 26.7277 4 25.3803 4 24.0001C4 21.9095 4.32077 19.8938 4.91579 17.9995C4.94381 17.9999 4.97188 18.0001 5 18.0001C8.31371 18.0001 11 15.3138 11 12.0001C11 11.0488 10.7786 10.1493 10.3846 9.35011C12.6975 7.1995 15.5205 5.59002 18.6521 4.72314C19.6444 6.66819 21.6667 8.00013 24 8.00013C26.3333 8.00013 28.3556 6.66819 29.3479 4.72314C32.4795 5.59002 35.3025 7.1995 37.6154 9.35011C37.2214 10.1493 37 11.0488 37 12.0001C37 15.3138 39.6863 18.0001 43 18.0001C43.0281 18.0001 43.0562 17.9999 43.0842 17.9995C43.6792 19.8938 44 21.9095 44 24.0001C44 25.3803 43.8602 26.7277 43.594 28.0292C43.3986 28.01 43.2005 28.0001 43 28.0001C39.6863 28.0001 37 30.6864 37 34.0001C37 35.4734 37.531 36.8227 38.4121 37.867C36.0502 40.3213 33.0673 42.1736 29.7162 43.1713C28.9428 40.752 26.676 39.0001 24 39.0001C21.324 39.0001 19.0572 40.752 18.2838 43.1713Z" />
      <path d="M24 31C27.866 31 31 27.866 31 24C31 20.134 27.866 17 24 17C20.134 17 17 20.134 17 24C17 27.866 20.134 31 24 31Z" />
    </svg>
  );
}

function DraftEditor({
  content,
  editorRef,
  edits,
  onAcceptEdit,
  onChange,
  onKeyDown,
  onRejectEdit,
  onSelectionChange,
}: {
  content: string;
  editorRef: React.RefObject<HTMLDivElement | null>;
  edits: DraftEdit[];
  onAcceptEdit: (id: string) => void;
  onChange: (content: string) => void;
  onKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void;
  onRejectEdit: (id: string) => void;
  onSelectionChange: () => void;
}) {
  const chunks = buildDraftSuggestionChunks(content, edits);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || edits.length) return;
    if (editor.innerText !== content) {
      editor.innerText = content;
    }
  }, [content, editorRef, edits.length]);

  return (
    <div
      ref={editorRef}
      className="draft-editor"
      contentEditable={!edits.length}
      role="textbox"
      aria-label="正文"
      spellCheck={false}
      suppressContentEditableWarning
      onInput={(event) => onChange(event.currentTarget.innerText)}
      onKeyDown={onKeyDown}
      onKeyUp={onSelectionChange}
      onMouseUp={onSelectionChange}
    >
      {chunks.map((chunk, index) => {
        if (chunk.kind === "text") {
          return <span key={`text-${index}`}>{chunk.text}</span>;
        }
        return (
          <span className="inline-edit" key={chunk.edit.id}>
            <span className="inline-diff">
              <del>{chunk.edit.target}</del>
              <ins>{chunk.edit.replacement}</ins>
            </span>
            {chunk.edit.rationale ? <small>{chunk.edit.rationale}</small> : null}
            <span className="inline-edit-actions" contentEditable={false}>
              <button type="button" onClick={() => onAcceptEdit(chunk.edit.id)}>确认</button>
              <button type="button" className="secondary" onClick={() => onRejectEdit(chunk.edit.id)}>取消</button>
            </span>
          </span>
        );
      })}
    </div>
  );
}

function readContentEditableSelection(root: HTMLElement): TextSelection | null {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0) return null;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.startContainer) || !root.contains(range.endContainer)) return null;
  const beforeStart = range.cloneRange();
  beforeStart.selectNodeContents(root);
  beforeStart.setEnd(range.startContainer, range.startOffset);
  const beforeEnd = range.cloneRange();
  beforeEnd.selectNodeContents(root);
  beforeEnd.setEnd(range.endContainer, range.endOffset);
  const start = beforeStart.toString().length;
  const end = beforeEnd.toString().length;
  return { start: Math.min(start, end), end: Math.max(start, end) };
}

function setContentEditableCaret(root: HTMLElement | null, offset: number) {
  if (!root) return;
  root.focus();
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let remaining = offset;
  let node = walker.nextNode();
  while (node) {
    const textLength = node.textContent?.length ?? 0;
    if (remaining <= textLength) {
      const range = document.createRange();
      range.setStart(node, remaining);
      range.collapse(true);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
      return;
    }
    remaining -= textLength;
    node = walker.nextNode();
  }
  const range = document.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}

type DraftSuggestionChunk =
  | { kind: "text"; text: string }
  | { kind: "edit"; edit: DraftEdit };

function buildDraftSuggestionChunks(content: string, edits: DraftEdit[]): DraftSuggestionChunk[] {
  const matches = createDraftReplaceGuardReport(content, edits).matches;
  const chunks: DraftSuggestionChunk[] = [];
  let cursor = 0;

  for (const match of matches) {
    if (match.index < cursor) continue;
    if (match.index > cursor) {
      chunks.push({ kind: "text", text: content.slice(cursor, match.index) });
    }
    chunks.push({ kind: "edit", edit: match.edit });
    cursor = match.index + match.edit.target.length;
  }

  if (cursor < content.length) {
    chunks.push({ kind: "text", text: content.slice(cursor) });
  }
  return chunks.length ? chunks : [{ kind: "text", text: content }];
}

function MemoryDrawer({
  currentTitle,
  extracting,
  memoryError,
  memoryState,
  onAcceptCandidate,
  onClose,
  onExtractCandidates,
  onIgnoreCandidate,
  onOpenMemoryFolder,
  onUpdateCandidate,
  workspace,
}: {
  currentTitle: string;
  extracting: boolean;
  memoryError: string;
  memoryState: MemoryState;
  onAcceptCandidate: (id: string) => void;
  onClose: () => void;
  onExtractCandidates: () => void;
  onIgnoreCandidate: (id: string) => void;
  onOpenMemoryFolder: () => void;
  onUpdateCandidate: (id: string, text: string) => void;
  workspace: WorkspaceInfo | null;
}) {
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer" role="dialog" aria-modal="true" aria-label="记忆" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">记忆</div>
            <div className="drawer-subtitle">当前文件：{currentTitle}</div>
          </div>
          <div className="drawer-header-actions">
            <button type="button" className="small-action" onClick={onOpenMemoryFolder}>
              文件夹
            </button>
            <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
              ×
            </button>
          </div>
        </div>

        {memoryError ? <div className="rail-error">{memoryError}</div> : null}

        <section className="memory-card">
          <h2>当前现场</h2>
          <p>{currentTitle}</p>
          <p>正文提取只会生成待确认记忆，由你决定是否写入。</p>
          <button type="button" className="extract-action" onClick={onExtractCandidates} disabled={extracting}>
            {extracting ? "提取中" : "从当前正文提取"}
          </button>
        </section>

        <section className="memory-card">
          <h2>相关记忆</h2>
          {memoryState.memories.length ? (
            <ul>
              {memoryState.memories.map((item) => (
                <li key={item.id}>
                  <span className="memory-category">{item.category ?? "其他"}</span>
                  {item.text}
                </li>
              ))}
            </ul>
          ) : (
            <p>还没有写入的记忆。</p>
          )}
        </section>

        {memoryState.candidates.length ? (
          memoryState.candidates.map((candidate) => (
            <MemoryCandidateCard
              candidate={candidate}
              key={candidate.id}
              onAccept={onAcceptCandidate}
              onIgnore={onIgnoreCandidate}
              onUpdate={onUpdateCandidate}
            />
          ))
        ) : (
          <section className="memory-card pending">
            <h2>待确认</h2>
            <p>暂无待确认记忆。</p>
          </section>
        )}

        <footer className="drawer-footer">
          {memoryState.memoryFolderPath || (workspace?.runtimePath ? `${workspace.runtimePath}` : "本地记忆目录初始化中")}
        </footer>
      </aside>
    </div>
  );
}

function MemoryCandidateCard({
  candidate,
  onAccept,
  onIgnore,
  onUpdate,
}: {
  candidate: MemoryCandidate;
  onAccept: (id: string) => void;
  onIgnore: (id: string) => void;
  onUpdate: (id: string, text: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(candidate.text);

  useEffect(() => {
    setDraft(candidate.text);
  }, [candidate.text]);

  const saveEdit = () => {
    const text = draft.trim();
    if (!text) return;
    onUpdate(candidate.id, text);
    setEditing(false);
  };

  return (
    <section className="memory-card pending">
      <h2>
        待确认
        <span className="memory-category">{candidate.category ?? "其他"}</span>
      </h2>
      {editing ? (
        <textarea className="candidate-editor" value={draft} onChange={(event) => setDraft(event.currentTarget.value)} aria-label="编辑候选记忆" />
      ) : (
        <p>{candidate.text}</p>
      )}
      <div className="drawer-actions">
        {editing ? (
          <>
            <button type="button" onClick={saveEdit}>
              保存
            </button>
            <button type="button" className="secondary" onClick={() => setEditing(false)}>
              取消
            </button>
          </>
        ) : (
          <>
            <button type="button" onClick={() => onAccept(candidate.id)}>
              记住
            </button>
            <button type="button" className="secondary" onClick={() => setEditing(true)}>
              编辑
            </button>
            <button type="button" className="secondary" onClick={() => onIgnore(candidate.id)}>
              忽略
            </button>
          </>
        )}
      </div>
    </section>
  );
}

function ModelSettingsDialog({ onClose }: { onClose: () => void }) {
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");
  const [maskedKey, setMaskedKey] = useState("");
  const [configured, setConfigured] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    void invoke<CustomApiSettingsStatus>("wridian_get_custom_api_settings")
      .then((status) => {
        setConfigured(status.configured);
        setBaseUrl(status.baseUrl ?? "");
        setModel(status.model ?? "");
        setMaskedKey(status.maskedKey ?? "");
      })
      .catch((error) => setMessage(error instanceof Error ? error.message : "请在 Wridian 桌面端配置模型账户。"));
  }, []);

  const saveSettings = async () => {
    setBusy(true);
    setMessage("");
    try {
      const status = await invoke<CustomApiSettingsStatus>("wridian_save_custom_api_settings", {
        input: { baseUrl, apiKey, model },
      });
      setConfigured(status.configured);
      setBaseUrl(status.baseUrl ?? "");
      setModel(status.model ?? "");
      setMaskedKey(status.maskedKey ?? "");
      setApiKey("");
      setMessage("已保存。");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const testSettings = async () => {
    setBusy(true);
    setMessage("");
    try {
      const response = await invoke<TestCustomApiResponse>("wridian_test_custom_api");
      setMessage(response.message || (response.ok ? "连接成功。" : "连接失败。"));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose} role="presentation">
      <section className="settings-dialog" role="dialog" aria-modal="true" aria-label="模型账户" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">模型账户</div>
            <div className="drawer-subtitle">先接入一个 OpenAI-compatible 第三方 API。</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="settings-form">
          <label>
            <span>Base URL</span>
            <input value={baseUrl} onChange={(event) => setBaseUrl(event.currentTarget.value)} placeholder="https://api.example.com/v1" />
          </label>
          <label>
            <span>API Key</span>
            <input
              value={apiKey}
              onChange={(event) => setApiKey(event.currentTarget.value)}
              placeholder={configured && maskedKey ? `已保存：${maskedKey}` : "sk-..."}
              type="password"
            />
          </label>
          <label>
            <span>Model</span>
            <input value={model} onChange={(event) => setModel(event.currentTarget.value)} placeholder="gpt-4o-mini" />
          </label>
        </div>

        {message ? <div className="settings-message">{message}</div> : null}

        <div className="settings-actions">
          <button type="button" className="secondary-action" onClick={() => void testSettings()} disabled={busy || !configured}>
            测试连接
          </button>
          <button type="button" className="primary-action" onClick={() => void saveSettings()} disabled={busy}>
            保存
          </button>
        </div>
      </section>
    </div>
  );
}

export default App;
