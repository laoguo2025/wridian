import { useState } from "react";
import type { WorkFileNode } from "../appTypes";

export type FileContextMenu = {
  node: WorkFileNode;
  x: number;
  y: number;
};

export function FileNodeView({
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

export function FileContextMenuView({
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
