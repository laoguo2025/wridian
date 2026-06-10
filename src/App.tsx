import { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  restorePromptPillsFromMessage,
  type ChatMessage,
} from "./chat/messageRepository";
import { useChatManager, type ChatDraftEdit } from "./chat/chatManager";
import { ChatPanel } from "./chat/ChatPanel";
import {
  getProjectState,
  selectProject,
  type ProjectState,
} from "./chat/projectContext";
import {
  buildPromptSuggestions,
  createReferencedFileContentPromptPill,
  createFileContentPromptPill,
  createFilePromptPill,
  createImagePromptPill,
  createPromptPillFromSuggestion,
  createSelectionPromptPill,
  upsertPromptContextPill,
  type DraftKind,
  type PromptContextPill,
} from "./chat/promptContext";
import {
  createDraftReplaceGuardReport,
  describeDraftReplaceSkip,
} from "./editor/draftReplaceGuard";
import { DraftEditor, readContentEditableSelection, setContentEditableCaret, type TextSelection } from "./editor/DraftEditor";
import { libraryFolderPath, libraryFolderTooltip } from "./libraryToolbar";
import {
  CREATIVE_SKILLS,
  DEFAULT_CREATIVE_SKILL_STATE,
  type CreativeSkill,
  type CreativeSkillId,
} from "./creativeSkills";
import { MemoryDrawer } from "./memory/MemoryDrawer";
import { KnowledgeGraphDrawer } from "./knowledge/KnowledgeGraphDrawer";
import { ModelSettingsDialog } from "./settings/ModelSettingsDialog";
import { clamp } from "./numberUtils";
import type {
  KnowledgeGraphState,
  MemoryTreeState,
  OpenFileResponse,
  SaveFileResponse,
  CustomApiSettingsStatus,
  WorkFileNode,
  WorkspaceInfo,
} from "./appTypes";
import "./App.css";

type Theme = "light" | "dark";
type FontSizeMode = "default" | "large" | "max";
type SaveStatus = "idle" | "dirty" | "saving" | "saved" | "error";

type DraftEdit = ChatDraftEdit;

type KnowledgeCategory = {
  detail: string;
  id: string;
  title: string;
};

type KnowledgeCardSuggestion = {
  category: string;
  categoryId: string;
  id: string;
  relativePath: string;
  sourcePath: string;
  title: string;
};

type FileContextMenu = {
  node: WorkFileNode;
  x: number;
  y: number;
};

const DEFAULT_LEFT_PANE_WIDTH = 218;
const DEFAULT_RIGHT_PANE_WIDTH = 292;
const MIN_LEFT_PANE_WIDTH = 168;
const MAX_LEFT_PANE_WIDTH = 360;
const MIN_RIGHT_PANE_WIDTH = 240;
const MAX_RIGHT_PANE_WIDTH = 420;
const MIN_WRITING_PANE_WIDTH = 360;
const WORKSPACE_RESIZER_WIDTH = 12;
const WORKSPACE_RESIZER_COUNT = 2;
const FONT_SIZE_SCALE: Record<FontSizeMode, number> = {
  default: 1,
  large: 1.12,
  max: 1.25,
};
function App() {
  const [theme, setTheme] = useState<Theme>("light");
  const [fontSizeMode, setFontSizeMode] = useState<FontSizeMode>("default");
  const [fontSizeMenuOpen, setFontSizeMenuOpen] = useState(false);
  const [memoryOpen, setMemoryOpen] = useState(false);
  const [knowledgeGraphOpen, setKnowledgeGraphOpen] = useState(false);
  const [creativeSkillsOpen, setCreativeSkillsOpen] = useState(false);
  const [knowledgeGraphState, setKnowledgeGraphState] = useState<KnowledgeGraphState>({ nodes: [], edges: [], warnings: [] });
  const [knowledgeGraphError, setKnowledgeGraphError] = useState("");
  const [creativeSkillEnabled, setCreativeSkillEnabled] = useState<Record<CreativeSkillId, boolean>>(DEFAULT_CREATIVE_SKILL_STATE);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [workspaceError, setWorkspaceError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [pendingEdits, setPendingEdits] = useState<DraftEdit[]>([]);
  const [promptPills, setPromptPills] = useState<PromptContextPill[]>([]);
  const [promptFileContentCache, setPromptFileContentCache] = useState<Record<string, string>>({});
  const [selectedKnowledgeCategoryId, setSelectedKnowledgeCategoryId] = useState("");
  const [activeModelLabel, setActiveModelLabel] = useState("");
  const [projectState, setProjectState] = useState<ProjectState>({ projects: [] });
  const [projectError, setProjectError] = useState("");
  const [hasDraftSelection, setHasDraftSelection] = useState(false);
  const [selectedPath, setSelectedPath] = useState("");
  const [loadingPath, setLoadingPath] = useState("");
  const [editorTitle, setEditorTitle] = useState("");
  const [editorContent, setEditorContent] = useState("");
  const [lastSavedContent, setLastSavedContent] = useState("");
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState("");
  const [memoryError, setMemoryError] = useState("");
  const [memoryTreeState, setMemoryTreeState] = useState<MemoryTreeState>({ roots: [] });
  const [savingMemoryTree, setSavingMemoryTree] = useState(false);
  const [fileMenu, setFileMenu] = useState<FileContextMenu | null>(null);
  const [libraryTab, setLibraryTab] = useState<"works" | "knowledge">("works");
  const [workspacePaneWidths, setWorkspacePaneWidths] = useState({
    left: DEFAULT_LEFT_PANE_WIDTH,
    right: DEFAULT_RIGHT_PANE_WIDTH,
  });
  const workspaceRef = useRef<HTMLDivElement | null>(null);
  const draftEditorRef = useRef<HTMLDivElement | null>(null);
  const fontSizeControlRef = useRef<HTMLDivElement | null>(null);
  const draftSelectionRef = useRef<TextSelection>({ start: editorContent.length, end: editorContent.length });
  const appendDraftEdits = useCallback((edits: DraftEdit[]) => {
    setPendingEdits((current) => [...current, ...edits]);
  }, []);

  useEffect(() => {
    if (!fontSizeMenuOpen) {
      return;
    }

    const closeFontSizeMenuOnOutsidePointerDown = (event: PointerEvent) => {
      const control = fontSizeControlRef.current;
      if (!control || !(event.target instanceof Node) || !control.contains(event.target)) {
        setFontSizeMenuOpen(false);
      }
    };

    document.addEventListener("pointerdown", closeFontSizeMenuOnOutsidePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", closeFontSizeMenuOnOutsidePointerDown, true);
    };
  }, [fontSizeMenuOpen]);
  const chatManager = useChatManager({ onDraftEdits: appendDraftEdits });
  const draftKind = useMemo(() => detectDraftKind(selectedPath, editorContent), [editorContent, selectedPath]);

  const loadMemoryTree = useCallback(async () => {
    try {
      const response = await invoke<MemoryTreeState>("wridian_get_memory_tree");
      setMemoryTreeState(response);
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const loadKnowledgeGraph = useCallback(async () => {
    try {
      const response = await invoke<KnowledgeGraphState>("wridian_get_knowledge_graph");
      setKnowledgeGraphState(response);
      setKnowledgeGraphError("");
    } catch (error) {
      setKnowledgeGraphError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const sendPrompt = async (override?: { text: string; selectedText?: string }) => {
    const userInput = (override?.text ?? prompt).trim();
    if (!userInput || chatManager.pending) return;
    if (!override) setPrompt("");
    setMemoryOpen(false);
    const sent = await chatManager.sendPrompt({
      content: editorContent,
      contextPills: override ? [] : promptPills,
      draftKind,
      selectedText: override?.selectedText,
      sourcePath: selectedPath,
      text: userInput,
      title: editorTitle,
    });
    if (sent && !override) {
      setPromptPills([]);
    }
  };

  const updateDraftSelection = useCallback(() => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const { start, end } = selection;
    draftSelectionRef.current = { start, end };
    setHasDraftSelection(end > start);
  }, []);

  const attachCurrentSelectionToPrompt = () => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const selected = editorContent.slice(selection.start, selection.end).trim();
    if (!selected) return;
    setPromptPills((current) => upsertPromptContextPill(current, createSelectionPromptPill(selected, selection)));
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
    if (!memoryOpen) return;
    void loadMemoryTree();
  }, [loadMemoryTree, memoryOpen]);

  useEffect(() => {
    if (!knowledgeGraphOpen) return;
    void loadKnowledgeGraph();
  }, [knowledgeGraphOpen, loadKnowledgeGraph, workspace?.knowledgeFiles.length]);

  useEffect(() => {
    void invoke<CustomApiSettingsStatus>("wridian_get_custom_api_settings")
      .then((status) => setActiveModelLabel(status.model ?? "未配置模型"))
      .catch(() => setActiveModelLabel("未配置模型"));
  }, []);

  useEffect(() => {
    void getProjectState()
      .then(setProjectState)
      .catch((error) => setProjectError(error instanceof Error ? error.message : String(error)));
  }, [workspace?.files.length, workspace?.filesRootPath]);

  const files = workspace?.files ?? [];
  const knowledgeFiles = workspace?.knowledgeFiles ?? [];
  const visibleFiles = libraryTab === "works" ? files : knowledgeFiles;
  const activeLibraryConfigured = libraryTab === "knowledge"
    ? Boolean(workspace?.knowledgeRootConfigured)
    : Boolean(workspace?.workRootConfigured);
  const isRealFile = Boolean(selectedPath);
  const dirty = isRealFile && !loadingPath && editorContent !== lastSavedContent;

  const saveCurrentFile = useCallback(async () => {
    if (!isRealFile || loadingPath || !dirty) return;
    const pathToSave = selectedPath;
    const contentToSave = editorContent;
    setSaveStatus("saving");
    setSaveError("");
    try {
      await invoke<SaveFileResponse>("wridian_save_file", {
        input: { path: pathToSave, content: contentToSave },
      });
      if (selectedPath === pathToSave && editorContent === contentToSave) {
        setLastSavedContent(contentToSave);
      }
      setSaveStatus("saved");
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  }, [dirty, editorContent, isRealFile, loadingPath, selectedPath]);

  useEffect(() => {
    if (!isRealFile || loadingPath) return;
    if (!dirty) {
      setSaveStatus("saved");
      return;
    }
    setSaveStatus("dirty");
    const timer = window.setTimeout(() => {
      void saveCurrentFile();
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [dirty, isRealFile, loadingPath, saveCurrentFile]);

  const switchProject = async (id: string) => {
    try {
      setProjectState(await selectProject(id || null));
      setProjectError("");
    } catch (error) {
      setProjectError(error instanceof Error ? error.message : String(error));
    }
  };

  const openCurrentLibraryFolder = async () => {
    setWorkspaceError("");
    const path = libraryFolderPath(libraryTab, workspace);
    if (!path) {
      await chooseLibraryRoot();
      return;
    }
    try {
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(path);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端打开本地文件夹。" : message);
    }
  };

  const refreshWorkspace = (response: WorkspaceInfo) => {
    setWorkspace(response);
    setWorkspaceError("");
  };

  const workspaceRootPath = libraryTab === "knowledge"
    ? activeLibraryConfigured ? workspace?.knowledgeRootPath || "" : ""
    : activeLibraryConfigured ? workspace?.filesRootPath || workspace?.activeWorkRoot || "" : "";

  const runWorkspaceAction = async (action: () => Promise<WorkspaceInfo>) => {
    setWorkspaceError("");
    try {
      refreshWorkspace(await action());
    } catch (error) {
      setWorkspaceError(error instanceof Error ? error.message : String(error));
    }
  };

  const chooseLibraryRoot = async (tab = libraryTab) => {
    setWorkspaceError("");
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: tab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹",
      });
      if (!selected || Array.isArray(selected)) return;
      const command = tab === "knowledge" ? "wridian_set_knowledge_root" : "wridian_set_work_root";
      refreshWorkspace(await invoke<WorkspaceInfo>(command, { input: { path: selected } }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端选择本地文件夹。" : message);
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
    void addFileToPrompt(node.name, node.path, node.relativePath);
  };

  const addFileToPrompt = async (name: string, path: string, relativePath = "") => {
    try {
      const cached = promptFileContentCache[path];
      const content = cached ?? (await invoke<OpenFileResponse>("wridian_open_file", { input: { path } })).content;
      setPromptFileContentCache((current) => ({ ...current, [path]: content }));
      setPromptPills((current) => upsertPromptContextPill(
        current,
        relativePath
          ? createReferencedFileContentPromptPill(name, path, relativePath, content)
          : createFileContentPromptPill(name, path, content),
      ));
    } catch {
      setPromptPills((current) => upsertPromptContextPill(current, createFilePromptPill(name, path, relativePath)));
    }
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

  const openFilePath = async (requestedPath: string, fallbackName = "") => {
    setLoadingPath(requestedPath);
    setEditorTitle(fallbackName || requestedPath.split(/[\\/]/).pop() || "未命名文件");
    setSaveError("");
    setSaveStatus("idle");
    try {
      const response = await invoke<OpenFileResponse>("wridian_open_file", { input: { path: requestedPath } });
      setSelectedPath(response.path);
      setEditorTitle(response.name);
      setEditorContent(response.content);
      setLastSavedContent(response.content);
      setPromptFileContentCache((current) => ({ ...current, [response.path]: response.content }));
      draftSelectionRef.current = { start: response.content.length, end: response.content.length };
      setHasDraftSelection(false);
      setPromptPills([]);
      setPendingEdits([]);
      setSaveStatus("saved");
      const project = projectState.projects.find((item) => response.path.startsWith(item.id));
      if (project && project.id !== projectState.activeProjectId) {
        void switchProject(project.id);
      } else if (!project && projectState.activeProjectId) {
        void switchProject("");
      }
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setLoadingPath((current) => (current === requestedPath ? "" : current));
    }
  };

  const openFile = async (node: WorkFileNode) => {
    if (node.folder) return;
    await openFilePath(node.path, node.name);
  };

  const openKnowledgeGraphFile = (path: string) => {
    setKnowledgeGraphOpen(false);
    void openFilePath(path);
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

  const editUserMessage = (message: ChatMessage) => {
    setPrompt(message.text);
    setPromptPills(restorePromptPillsFromMessage(message));
  };

  const retryLastUserMessage = (message: ChatMessage) => {
    setPromptPills(restorePromptPillsFromMessage(message));
    void sendPrompt({ text: message.text, selectedText: message.selectedText });
  };

  const acceptEdit = (id: string) => {
    const edit = pendingEdits.find((item) => item.id === id && item.status === "pending");
    if (!edit) return;
    const guardReport = createDraftReplaceGuardReport(editorContent, pendingEdits.filter((item) => item.status === "pending"));
    const match = guardReport.matches.find((item) => item.edit.id === id);
    if (!match) {
      const skipped = guardReport.skipped.find((item) => item.edit.id === id);
      chatManager.setError(skipped ? describeDraftReplaceSkip(skipped.reason) : "这处修改无法安全定位。");
      return;
    }
    chatManager.setError("");
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
      chatManager.setError("没有可以安全确认的修改。");
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
    chatManager.setError(guardReport.skipped.length ? `${guardReport.skipped.length} 处修改需要重新定位。` : "");
    draftSelectionRef.current = { start: 0, end: 0 };
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
  const knowledgeSuggestionIndex = useMemo(() => buildKnowledgeSuggestionIndex(knowledgeFiles), [knowledgeFiles]);
  const enabledCreativeSkills = useMemo(
    () => CREATIVE_SKILLS.filter((skill) => creativeSkillEnabled[skill.id]),
    [creativeSkillEnabled],
  );
  const promptSuggestions = useMemo(() => buildPromptSuggestions({
    creativeSkills: enabledCreativeSkills,
    draftKind,
    knowledgeCards: knowledgeSuggestionIndex.cards,
    knowledgeCategories: knowledgeSuggestionIndex.categories,
    selectedKnowledgeCategoryId,
  }), [draftKind, enabledCreativeSkills, knowledgeSuggestionIndex, selectedKnowledgeCategoryId]);

  const statusLabel = useMemo(() => {
    if (saveStatus === "idle") return "读取中";
    if (saveStatus === "dirty") return "未保存";
    if (saveStatus === "saving") return "正在保存";
    if (saveStatus === "error") return "保存失败";
    return "已保存";
  }, [saveStatus]);

  const saveMemoryTreeFile = async (path: string, content: string): Promise<boolean> => {
    setSavingMemoryTree(true);
    try {
      const response = await invoke<MemoryTreeState>("wridian_save_memory_tree_file", {
        input: { path, content },
      });
      setMemoryTreeState(response);
      setMemoryError("");
      return true;
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
      return false;
    } finally {
      setSavingMemoryTree(false);
    }
  };

  const deleteMemoryTreeFile = async (path: string): Promise<boolean> => {
    setSavingMemoryTree(true);
    try {
      const response = await invoke<MemoryTreeState>("wridian_delete_memory_tree_file", {
        input: { path },
      });
      setMemoryTreeState(response);
      setMemoryError("");
      return true;
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
      return false;
    } finally {
      setSavingMemoryTree(false);
    }
  };

  const resizeWorkspacePane = (side: "left" | "right", event: ReactPointerEvent<HTMLDivElement>) => {
    const workspaceNode = workspaceRef.current;
    if (!workspaceNode) return;
    event.preventDefault();

    const startX = event.clientX;
    const startWidths = workspacePaneWidths;
    const workspaceWidth = workspaceNode.getBoundingClientRect().width;
    const resizerSpace = WORKSPACE_RESIZER_WIDTH * WORKSPACE_RESIZER_COUNT;

    const onPointerMove = (moveEvent: PointerEvent) => {
      const deltaX = moveEvent.clientX - startX;
      setWorkspacePaneWidths(() => {
        if (side === "left") {
          const maxLeft = Math.min(
            MAX_LEFT_PANE_WIDTH,
            workspaceWidth - startWidths.right - MIN_WRITING_PANE_WIDTH - resizerSpace,
          );
          return {
            left: clamp(startWidths.left + deltaX, MIN_LEFT_PANE_WIDTH, maxLeft),
            right: startWidths.right,
          };
        }

        const maxRight = Math.min(
          MAX_RIGHT_PANE_WIDTH,
          workspaceWidth - startWidths.left - MIN_WRITING_PANE_WIDTH - resizerSpace,
        );
        return {
          left: startWidths.left,
          right: clamp(startWidths.right - deltaX, MIN_RIGHT_PANE_WIDTH, maxRight),
        };
      });
    };

    const onPointerUp = () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp, { once: true });
  };

  const openMemoryFolder = async () => {
    if (!workspace?.runtimePath) {
      setMemoryError("请在 Wridian 桌面端打开记忆文件夹。");
      return;
    }
    try {
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(`${workspace.runtimePath}\\memory-tree`);
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div
      className="app-shell"
      style={{ "--ui-font-scale": FONT_SIZE_SCALE[fontSizeMode] } as React.CSSProperties}
    >
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" />
          <span>Wridian</span>
        </div>
        <nav className="top-actions" aria-label="Wridian actions">
          <button type="button" title="创作记忆树" aria-label="创作记忆树" onClick={() => {
            setMemoryOpen(true);
          }}>
            <MemoryTreeIcon />
          </button>
          <button type="button" title="知识图谱" aria-label="知识图谱" onClick={() => setKnowledgeGraphOpen(true)}>
            <KnowledgeGraphIcon />
          </button>
          <button type="button" title="技能管理" aria-label="技能管理" onClick={() => setCreativeSkillsOpen(true)}>
            <LightningIcon />
          </button>
          <button type="button" title="模型配置" aria-label="模型配置" onClick={() => setSettingsOpen(true)}>
            <ModelConfigIcon />
          </button>
          <div className="font-size-control" ref={fontSizeControlRef}>
            <button
              type="button"
              title="字体大小"
              aria-label="字体大小"
              aria-expanded={fontSizeMenuOpen}
              onClick={() => setFontSizeMenuOpen((open) => !open)}
            >
              <FontSizeIcon />
            </button>
            {fontSizeMenuOpen ? (
              <div className="font-size-popover" role="menu" aria-label="字体大小">
                <button type="button" className={fontSizeMode === "default" ? "active" : ""} onClick={() => {
                  setFontSizeMode("default");
                  setFontSizeMenuOpen(false);
                }}>
                  默认
                </button>
                <button type="button" className={fontSizeMode === "large" ? "active" : ""} onClick={() => {
                  setFontSizeMode("large");
                  setFontSizeMenuOpen(false);
                }}>
                  较大
                </button>
                <button type="button" className={fontSizeMode === "max" ? "active" : ""} onClick={() => {
                  setFontSizeMode("max");
                  setFontSizeMenuOpen(false);
                }}>
                  最大
                </button>
              </div>
            ) : null}
          </div>
          <button
            type="button"
            title={theme === "light" ? "深色主题" : "浅色主题"}
            aria-label={theme === "light" ? "深色主题" : "浅色主题"}
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
          >
            {theme === "light" ? <DarkThemeIcon /> : <LightThemeIcon />}
          </button>
        </nav>
      </header>

      <div
        className="workspace"
        ref={workspaceRef}
        style={{
          "--left-pane-width": `${workspacePaneWidths.left}px`,
          "--right-pane-width": `${workspacePaneWidths.right}px`,
        } as React.CSSProperties}
      >
        <aside className="project-rail" aria-label="作品">
          <div className="rail-topline">
            <div className="library-tabs" role="tablist" aria-label="资料库">
              <button type="button" className={libraryTab === "works" ? "active" : ""} onClick={() => setLibraryTab("works")}>
                作品库
              </button>
              <button type="button" className={libraryTab === "knowledge" ? "active" : ""} onClick={() => setLibraryTab("knowledge")}>
                知识库
              </button>
            </div>
            <div className="file-toolbar" aria-label="文件操作">
              <button type="button" title="新建文件" aria-label="新建文件" onClick={() => void createFile()} disabled={!activeLibraryConfigured}>
                <PencilIcon />
              </button>
              <button type="button" title="新建文件夹" aria-label="新建文件夹" onClick={() => void createFolder()} disabled={!activeLibraryConfigured}>
                <FolderPlusIcon />
              </button>
              <button
                type="button"
                title={activeLibraryConfigured ? libraryFolderTooltip(libraryTab) : libraryTab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹"}
                aria-label={activeLibraryConfigured ? libraryFolderTooltip(libraryTab) : libraryTab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹"}
                onClick={() => void openCurrentLibraryFolder()}
              >
                <WorkFolderIcon />
              </button>
            </div>
          </div>
          {workspaceError ? <div className="rail-error">{workspaceError}</div> : null}

          <div className="file-tree">
            {visibleFiles.length ? (
              visibleFiles.map((node) => (
                <FileNodeView
                  key={node.path}
                  node={node}
                  depth={0}
                  selectedPath={selectedPath}
                  onOpenFile={openFile}
                  onOpenMenu={openFileContextMenu}
                />
              ))
            ) : null}
          </div>

          <div className="rail-bottom">
            <button type="button" title="系统设置" aria-label="系统设置" onClick={() => setSettingsOpen(true)}>
              <SettingsIcon />
            </button>
          </div>
        </aside>

        <div
          className="workspace-resizer"
          role="separator"
          aria-label="调整作品区宽度"
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={(event) => resizeWorkspacePane("left", event)}
        />

        <main className="writing-pane">
          <section className={`paper ${selectedPath ? "" : "paper-empty"}`} aria-label="正文编辑区">
            {selectedPath ? (
              <div className="paper-topline">
                <div className="paper-kicker">{baseName(selectedPath)}</div>
                <div className="paper-actions">
                  <button type="button" className="paper-action" onClick={attachCurrentSelectionToPrompt} disabled={!hasDraftSelection}>
                    添加选区到输入框
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

        <div
          className="workspace-resizer"
          role="separator"
          aria-label="调整对话区宽度"
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={(event) => resizeWorkspacePane("right", event)}
        />

        <ChatPanel
          error={chatManager.error}
          messages={chatManager.messages}
          onCopy={copyText}
          onEditUserMessage={editUserMessage}
          onRetry={retryLastUserMessage}
          pending={chatManager.pending}
          prompt={prompt}
          promptPills={promptPills}
          promptSuggestions={promptSuggestions}
          activeModelLabel={activeModelLabel}
          projectError={projectError}
          projects={projectState.projects}
          selectedProjectId={projectState.activeProjectId ?? ""}
          onSelectProject={(id) => void switchProject(id)}
          onPromptChange={setPrompt}
          onPromptPillsChange={setPromptPills}
          onImagePaste={(files) => {
            setPromptPills((current) => files.reduce(
              (next, file) => upsertPromptContextPill(next, createImagePromptPill(file.name || "pasted-image", file.size)),
              current,
            ));
          }}
          onRemovePill={(id) => setPromptPills((current) => current.filter((pill) => pill.id !== id))}
          onSelectSuggestion={(suggestion) => {
            if (suggestion.kind !== "context") return;
            if (suggestion.id.startsWith("knowledge-category:")) {
              const categoryId = suggestion.insertText.slice("category:".length);
              setSelectedKnowledgeCategoryId(categoryId);
              setPrompt("@");
              return;
            }
            if (suggestion.pillKind === "memory" && suggestion.insertText.startsWith("path:")) {
              const path = suggestion.insertText.slice("path:".length);
              void addFileToPrompt(suggestion.label, path, suggestion.relativePath ?? "");
              setSelectedKnowledgeCategoryId("");
              return;
            }
            setPromptPills((current) => upsertPromptContextPill(current, createPromptPillFromSuggestion(suggestion)));
          }}
          onSubmit={() => void sendPrompt()}
        />
      </div>

      {memoryOpen ? (
        <MemoryDrawer
          memoryError={memoryError}
          memoryTree={memoryTreeState}
          onClose={() => setMemoryOpen(false)}
          onDeleteFile={deleteMemoryTreeFile}
          onOpenMemoryFolder={openMemoryFolder}
          onSaveFile={saveMemoryTreeFile}
          saving={savingMemoryTree}
        />
      ) : null}
      {knowledgeGraphOpen ? (
        <KnowledgeGraphDrawer
          graph={knowledgeGraphState}
          graphError={knowledgeGraphError}
          knowledgeRootConfigured={Boolean(workspace?.knowledgeRootConfigured)}
          onClose={() => setKnowledgeGraphOpen(false)}
          onOpenFile={openKnowledgeGraphFile}
          onRefresh={loadKnowledgeGraph}
        />
      ) : null}
      {creativeSkillsOpen ? (
        <CreativeSkillsDrawer
          enabled={creativeSkillEnabled}
          onClose={() => setCreativeSkillsOpen(false)}
          onToggle={(id) => {
            setCreativeSkillEnabled((current) => ({ ...current, [id]: !current[id] }));
          }}
          skills={CREATIVE_SKILLS}
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
        title={node.relativePath || node.name}
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
        添加到对话输入
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

function detectDraftKind(path: string, content: string): DraftKind {
  const lowerPath = path.toLowerCase();
  if (lowerPath.endsWith(".fountain")) return "screenplay";

  const sceneSignals = (content.match(/(^|\n)\s*(INT\.|EXT\.|内景|外景|第[一二三四五六七八九十\d]+[集场])/g) ?? []).length;
  const dialogueSignals = (content.match(/(^|\n)\s*[\u4e00-\u9fa5A-Za-z0-9_]{2,12}[：:]/g) ?? []).length;
  return sceneSignals >= 2 || dialogueSignals >= 4 ? "screenplay" : "prose";
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

function MemoryTreeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M13.0448 14C13.5501 8.3935 18.262 4 24 4C29.738 4 34.4499 8.3935 34.9552 14H35C39.9706 14 44 18.0294 44 23C44 27.9706 39.9706 32 35 32H13C8.02944 32 4 27.9706 4 23C4 18.0294 8.02944 14 13 14H13.0448Z" />
      <path d="M24 28L29 23" />
      <path d="M24 25L18 19" />
      <path d="M24 44V18" />
    </svg>
  );
}

function KnowledgeGraphIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M13.5 39.3706C16.3908 41.6439 20.0371 42.9999 24 42.9999C27.9629 42.9999 31.6092 41.6439 34.5 39.3706" />
      <path d="M19 9.74707C12.0513 11.8822 7 18.3511 7 25.9999C7 27.9247 7.31989 29.7748 7.9094 31.4999" />
      <path d="M29 9.74707C35.9487 11.8822 41 18.3511 41 25.9999C41 27.9247 40.6801 29.7748 40.0906 31.4999" />
      <path d="M43 36C43 37.3416 42.4716 38.5597 41.6117 39.4577C40.7015 40.4082 39.4199 41 38 41C35.2386 41 33 38.7614 33 36C33 33.9899 34.1861 32.2569 35.8967 31.4626C36.536 31.1657 37.2487 31 38 31C40.7614 31 43 33.2386 43 36Z" />
      <path d="M15 36C15 37.3416 14.4716 38.5597 13.6117 39.4577C12.7015 40.4082 11.4199 41 10 41C7.23858 41 5 38.7614 5 36C5 33.9899 6.18614 32.2569 7.89667 31.4626C8.53604 31.1657 9.24867 31 10 31C12.7614 31 15 33.2386 15 36Z" />
      <path d="M29 9C29 10.3416 28.4716 11.5597 27.6117 12.4577C26.7015 13.4082 25.4199 14 24 14C21.2386 14 19 11.7614 19 9C19 6.98991 20.1861 5.25686 21.8967 4.4626C22.536 4.16572 23.2487 4 24 4C26.7614 4 29 6.23858 29 9Z" />
    </svg>
  );
}

function LightningIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M27 4L10 27H22L19 44L38 19H25L27 4Z" />
    </svg>
  );
}

function ModelConfigIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M18 23.9372V10C18 6.68629 20.6863 4 24 4C27.3137 4 30 6.68629 30 10V12.0057" />
      <path d="M30 24.0034V37.9999C30 41.3136 27.3137 43.9999 24 43.9999C20.6863 43.9999 18 41.3136 18 37.9999V35.9699" />
      <path d="M24 30H9.98415C6.67919 30 4 27.3137 4 24C4 20.6863 6.67919 18 9.98415 18H11.9886" />
      <path d="M24 18H37.9888C41.3087 18 44 20.6863 44 24C44 27.3137 41.3087 30 37.9888 30H36.0663" />
    </svg>
  );
}

function FontSizeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M4 8H32" />
      <path d="M28 21H44" />
      <path d="M18 42L18 8" />
      <path d="M36 42L36 21" />
    </svg>
  );
}

function LightThemeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M24 37C31.1797 37 37 31.1797 37 24C37 16.8203 31.1797 11 24 11C16.8203 11 11 16.8203 11 24C11 31.1797 16.8203 37 24 37Z" />
      <circle cx="24" cy="3.5" r="2.5" />
      <circle cx="38.5" cy="9.5" r="2.5" />
      <circle cx="44.5" cy="24" r="2.5" />
      <circle cx="38.5" cy="38.5" r="2.5" />
      <circle cx="24" cy="44.5" r="2.5" />
      <circle cx="9.5" cy="38.5" r="2.5" />
      <circle cx="3.5" cy="24" r="2.5" />
      <circle cx="9.5" cy="9.5" r="2.5" />
    </svg>
  );
}

function DarkThemeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M28.0527 4.41085C22.5828 5.83695 18.5455 10.8106 18.5455 16.7273C18.5455 23.7564 24.2436 29.4545 31.2727 29.4545C37.1894 29.4545 42.1631 25.4172 43.5891 19.9473C43.8585 21.256 44 22.6115 44 24C44 35.0457 35.0457 44 24 44C12.9543 44 4 35.0457 4 24C4 12.9543 12.9543 4 24 4C25.3885 4 26.744 4.14149 28.0527 4.41085Z" />
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

function CreativeSkillsDrawer({
  enabled,
  onClose,
  onToggle,
  skills,
}: {
  enabled: Record<CreativeSkillId, boolean>;
  onClose: () => void;
  onToggle: (id: CreativeSkillId) => void;
  skills: CreativeSkill[];
}) {
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer creative-skills-drawer" role="dialog" aria-modal="true" aria-label="技能管理" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">技能管理</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="creative-skills-list">
          {skills.map((skill) => (
            <div className="creative-skill-row" key={skill.id}>
              <div className="creative-skill-main">
                <div className="creative-skill-title">{skill.title}</div>
                <div className="creative-skill-meta">{skill.status}</div>
              </div>
              <button
                type="button"
                className={enabled[skill.id] ? "skill-toggle active" : "skill-toggle"}
                aria-pressed={enabled[skill.id]}
                onClick={() => onToggle(skill.id)}
              >
                <span />
              </button>
            </div>
          ))}
        </div>
      </aside>
    </div>
  );
}

function buildKnowledgeSuggestionIndex(nodes: WorkFileNode[]) {
  const categories = new Map<string, KnowledgeCategory>();
  const cards: KnowledgeCardSuggestion[] = [];
  const visit = (node: WorkFileNode, categoryId = "") => {
    if (node.folder) {
      const id = node.relativePath || node.path;
      categories.set(id, {
        detail: `${countMarkdownCards(node)} 张知识卡`,
        id,
        title: node.name,
      });
      node.children.forEach((child) => visit(child, id));
      return;
    }
    if (!/\.(md|markdown)$/i.test(node.name)) return;
    const fallbackCategory = categoryId || "__root__";
    if (!categoryId && !categories.has(fallbackCategory)) {
      categories.set(fallbackCategory, {
        detail: "知识库根目录",
        id: fallbackCategory,
        title: "根目录",
      });
    }
    cards.push({
      category: categories.get(fallbackCategory)?.title ?? "知识卡",
      categoryId: fallbackCategory,
      id: node.path,
      relativePath: node.relativePath,
      sourcePath: node.path,
      title: node.name.replace(/\.(md|markdown)$/i, ""),
    });
  };
  nodes.forEach((node) => visit(node));
  return {
    cards,
    categories: [...categories.values()].filter((category) =>
      cards.some((card) => card.categoryId === category.id),
    ),
  };
}

function countMarkdownCards(node: WorkFileNode): number {
  if (!node.folder) return /\.(md|markdown)$/i.test(node.name) ? 1 : 0;
  return node.children.reduce((total, child) => total + countMarkdownCards(child), 0);
}

export default App;
