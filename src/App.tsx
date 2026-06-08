import { KeyboardEvent, useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Theme = "light" | "dark";
type SaveStatus = "demo" | "idle" | "dirty" | "saving" | "saved" | "error";

type WorkspaceInfo = {
  vaultPath: string;
  runtimePath: string;
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

const fallbackFiles: WorkFileNode[] = [
  {
    name: "雾城手记",
    path: "demo://雾城手记",
    folder: true,
    children: [
      { name: "outline.md", path: "demo://outline.md", folder: false, children: [] },
      {
        name: "chapters",
        path: "demo://chapters",
        folder: true,
        children: [
          { name: "01.md", path: "demo://chapters/01.md", folder: false, children: [] },
          { name: "02.md", path: "demo://chapters/02.md", folder: false, children: [] },
          { name: "03.md", path: "demo://chapters/03.md", folder: false, children: [] },
        ],
      },
      {
        name: "characters",
        path: "demo://characters",
        folder: true,
        children: [{ name: "女主.md", path: "demo://characters/女主.md", folder: false, children: [] }],
      },
      { name: "world.md", path: "demo://world.md", folder: false, children: [] },
    ],
  },
];

const demoContent: Record<string, string> = {
  "demo://outline.md": "# 雾城手记\n\n## 核心悬念\n\n女主回到旧楼，发现父亲失踪前留下的线索。\n\n## 当前推进\n\n第三章需要强化她主动进门的理由。",
  "demo://chapters/01.md": "第一章\n\n雨从黄昏开始下。她在车站站了很久，直到最后一班公交车亮着灯驶出站台。",
  "demo://chapters/02.md": "第二章\n\n电话里的人只说了一句话：不要回那栋楼。",
  "demo://chapters/03.md": [
    "她推开门的时候，雨水顺着袖口往下滴。",
    "",
    "屋里没有开灯，只有楼道里的光斜斜切进来。她没有立刻喊人。那一秒的停顿不像恐惧，更像确认。",
    "",
    "她已经知道里面会有什么。",
  ].join("\n"),
  "demo://characters/女主.md": "# 女主\n\n- 不轻易解释自己的判断。\n- 对父亲的失踪有长期愧疚。\n- 第三章进入房间不是冲动，而是确认。",
  "demo://world.md": "# 世界设定\n\n雾城常年潮湿，旧楼区在十年前的事故后逐步空置。",
};

const memoryItems = [
  "女主知道父亲失踪真相，但还没有证据。",
  "雨夜场景不能提前暴露凶手。",
  "第三章要强化她进门前的主动选择。",
];

function App() {
  const [theme, setTheme] = useState<Theme>("light");
  const [memoryOpen, setMemoryOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [workspaceError, setWorkspaceError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [selectedPath, setSelectedPath] = useState("demo://chapters/03.md");
  const [editorTitle, setEditorTitle] = useState("03.md");
  const [editorContent, setEditorContent] = useState(demoContent["demo://chapters/03.md"]);
  const [lastSavedContent, setLastSavedContent] = useState(demoContent["demo://chapters/03.md"]);
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("demo");
  const [saveError, setSaveError] = useState("");

  const sendPrompt = () => {
    setPrompt("");
    setMemoryOpen(true);
  };

  const handlePromptKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    sendPrompt();
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

  const files = workspace?.files?.length ? workspace.files : fallbackFiles;
  const isRealFile = selectedPath && !selectedPath.startsWith("demo://");
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

  const openFile = async (node: WorkFileNode) => {
    if (node.folder) return;
    setSelectedPath(node.path);
    setEditorTitle(node.name);
    setSaveError("");
    if (node.path.startsWith("demo://")) {
      const content = demoContent[node.path] ?? "";
      setEditorContent(content);
      setLastSavedContent(content);
      setSaveStatus("demo");
      return;
    }
    setSaveStatus("idle");
    try {
      const response = await invoke<OpenFileResponse>("wridian_open_file", { input: { path: node.path } });
      setSelectedPath(response.path);
      setEditorTitle(response.name);
      setEditorContent(response.content);
      setLastSavedContent(response.content);
      setSaveStatus("saved");
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleDraftKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveCurrentFile();
    }
  };

  const statusLabel = useMemo(() => {
    if (saveStatus === "demo") return "示例";
    if (saveStatus === "idle") return "读取中";
    if (saveStatus === "dirty") return "未保存";
    if (saveStatus === "saving") return "正在保存";
    if (saveStatus === "error") return "保存失败";
    return "已保存";
  }, [saveStatus]);

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" />
          <span>Wridian</span>
        </div>
        <nav className="top-actions" aria-label="Wridian actions">
          <button type="button" onClick={() => setMemoryOpen(true)}>
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
            <button className="open-folder" type="button" onClick={() => void openWorkFolder()}>
              打开文件夹
            </button>
          </div>
          <button className="work-title active" type="button" title={workspace?.activeWorkRoot || workspace?.vaultPath || undefined}>
            <span>{workspace?.activeWorkRoot ? baseName(workspace.activeWorkRoot) : "Wridian Vault"}</span>
            <small>{workspace?.activeWorkRoot || "默认本地写作目录"}</small>
          </button>

          {workspaceError ? <div className="rail-error">{workspaceError}</div> : null}

          <div className="file-tree">
            {files.map((node) => (
              <FileNodeView key={node.path} node={node} depth={0} selectedPath={selectedPath} onOpenFile={openFile} />
            ))}
          </div>

          <div className="rail-divider" />
          <button className="new-work" type="button">
            + 新建
          </button>
        </aside>

        <main className="writing-pane">
          <section className="paper" aria-label="正文编辑区">
            <div className="paper-topline">
              <div className="paper-kicker">{selectedPath.startsWith("demo://") ? "示例作品" : baseName(selectedPath)}</div>
              <div className={`save-state ${saveStatus}`} title={saveError || undefined}>
                {statusLabel}
              </div>
            </div>
            <h1 className="chapter-heading">{editorTitle}</h1>
            <textarea
              className="draft-editor"
              value={editorContent}
              onChange={(event) => setEditorContent(event.currentTarget.value)}
              onKeyDown={handleDraftKeyDown}
              aria-label="正文"
              spellCheck={false}
            />
            {saveError ? <div className="paper-error">{saveError}</div> : null}
          </section>

          <form
            className="prompt-bar"
            onSubmit={(event) => {
              event.preventDefault();
              sendPrompt();
            }}
          >
            <textarea
              value={prompt}
              onChange={(event) => setPrompt(event.currentTarget.value)}
              onKeyDown={handlePromptKeyDown}
              placeholder="Enter 发送，Shift + Enter 换行"
              aria-label="共创输入"
            />
            <button type="submit" aria-label="发送">
              ↑
            </button>
          </form>
        </main>
      </div>

      {memoryOpen ? <MemoryDrawer onClose={() => setMemoryOpen(false)} workspace={workspace} /> : null}
      {settingsOpen ? <ModelSettingsDialog onClose={() => setSettingsOpen(false)} /> : null}
    </div>
  );
}

function FileNodeView({
  depth,
  node,
  onOpenFile,
  selectedPath,
}: {
  depth: number;
  node: WorkFileNode;
  onOpenFile: (node: WorkFileNode) => void;
  selectedPath: string;
}) {
  return (
    <div className="file-node">
      <button
        className={node.folder ? "file-row folder" : node.path === selectedPath ? "file-row active" : "file-row"}
        type="button"
        style={{ paddingLeft: `${8 + depth * 12}px` }}
        title={node.path}
        onClick={() => onOpenFile(node)}
      >
        <span>{node.folder ? "▾" : ""}</span>
        <strong>{node.name}</strong>
      </button>
      {node.folder && node.children.length ? (
        <div className="file-children">
          {node.children.map((child) => (
            <FileNodeView key={child.path} node={child} depth={depth + 1} selectedPath={selectedPath} onOpenFile={onOpenFile} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function baseName(path: string) {
  return path.replace(/[\\/]+$/g, "").split(/[\\/]/).pop() || path;
}

function MemoryDrawer({ onClose, workspace }: { onClose: () => void; workspace: WorkspaceInfo | null }) {
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer" role="dialog" aria-modal="true" aria-label="记忆" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">记忆</div>
            <div className="drawer-subtitle">当前作品：雾城手记</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <section className="memory-card">
          <h2>当前现场</h2>
          <p>第三章：雨夜</p>
          <p>上次讨论：女主进门前的动机。</p>
        </section>

        <section className="memory-card">
          <h2>相关记忆</h2>
          <ul>
            {memoryItems.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </section>

        <section className="memory-card pending">
          <h2>待确认</h2>
          <p>第三章需要让女主动机提前出现。</p>
          <div className="drawer-actions">
            <button type="button">记住</button>
            <button type="button" className="secondary">
              忽略
            </button>
          </div>
        </section>

        <footer className="drawer-footer">{workspace?.vaultPath ? `Vault: ${workspace.vaultPath}` : "本地 Vault 初始化中"}</footer>
      </aside>
    </div>
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
