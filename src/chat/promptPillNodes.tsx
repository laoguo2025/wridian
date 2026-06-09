import type { ReactNode } from "react";
import {
  $createTextNode,
  $getRoot,
  DecoratorNode,
  type DOMConversionMap,
  type DOMConversionOutput,
  type DOMExportOutput,
  type EditorConfig,
  type LexicalEditor,
  type LexicalNode,
  type NodeKey,
  type SerializedLexicalNode,
} from "lexical";
import type { PromptContextPill, PromptContextPillKind } from "./promptContext";

export interface SerializedPromptPillNode extends SerializedLexicalNode {
  id: string;
  kind: PromptContextPillKind;
  label: string;
  relativePath?: string;
  sourcePath?: string;
  value: string;
}

export class PromptPillNode extends DecoratorNode<ReactNode> {
  __id: string;
  __kind: PromptContextPillKind;
  __label: string;
  __relativePath?: string;
  __sourcePath?: string;
  __value: string;

  static getType(): string {
    return "wridian-prompt-pill";
  }

  static clone(node: PromptPillNode): PromptPillNode {
    return new PromptPillNode(
      node.__id,
      node.__kind,
      node.__label,
      node.__value,
      node.__sourcePath,
      node.__relativePath,
      node.__key,
    );
  }

  static importJSON(serializedNode: SerializedPromptPillNode): PromptPillNode {
    return $createPromptPillNode({
      id: serializedNode.id,
      kind: serializedNode.kind,
      label: serializedNode.label,
      relativePath: serializedNode.relativePath,
      sourcePath: serializedNode.sourcePath,
      value: serializedNode.value,
    });
  }

  static importDOM(): DOMConversionMap | null {
    return {
      span: (node: HTMLElement) => {
        if (!node.hasAttribute("data-wridian-prompt-pill")) return null;
        return {
          conversion: convertPromptPillElement,
          priority: 1,
        };
      },
    };
  }

  constructor(
    id: string,
    kind: PromptContextPillKind,
    label: string,
    value: string,
    sourcePath?: string,
    relativePath?: string,
    key?: NodeKey,
  ) {
    super(key);
    this.__id = id;
    this.__kind = kind;
    this.__label = label;
    this.__relativePath = relativePath;
    this.__sourcePath = sourcePath;
    this.__value = value;
  }

  createDOM(_config: EditorConfig, editor: LexicalEditor): HTMLElement {
    const span = editor.getRootElement()?.ownerDocument.createElement("span") ?? document.createElement("span");
    span.className = `prompt-editor-pill pill-${this.__kind}`;
    return span;
  }

  updateDOM(): false {
    return false;
  }

  exportDOM(editor: LexicalEditor): DOMExportOutput {
    const element = editor.getRootElement()?.ownerDocument.createElement("span") ?? document.createElement("span");
    element.setAttribute("data-wridian-prompt-pill", "true");
    element.setAttribute("data-pill-id", this.__id);
    element.setAttribute("data-pill-kind", this.__kind);
    element.setAttribute("data-pill-label", this.__label);
    element.setAttribute("data-pill-value", this.__value);
    if (this.__relativePath) {
      element.setAttribute("data-pill-relative-path", this.__relativePath);
    }
    if (this.__sourcePath) {
      element.setAttribute("data-pill-source-path", this.__sourcePath);
    }
    element.textContent = this.__label;
    return { element };
  }

  exportJSON(): SerializedPromptPillNode {
    return {
      ...super.exportJSON(),
      id: this.__id,
      kind: this.__kind,
      label: this.__label,
      relativePath: this.__relativePath,
      sourcePath: this.__sourcePath,
      type: "wridian-prompt-pill",
      value: this.__value,
      version: 1,
    };
  }

  getTextContent(): string {
    return `@${this.__label}`;
  }

  isInline(): boolean {
    return true;
  }

  canInsertTextBefore(): boolean {
    return true;
  }

  canInsertTextAfter(): boolean {
    return true;
  }

  canBeEmpty(): boolean {
    return false;
  }

  isKeyboardSelectable(): boolean {
    return true;
  }

  isIsolated(): boolean {
    return true;
  }

  getPill(): PromptContextPill {
    return {
      id: this.__id,
      kind: this.__kind,
      label: this.__label,
      relativePath: this.__relativePath,
      sourcePath: this.__sourcePath,
      value: this.__value,
    };
  }

  decorate(): ReactNode {
    return (
      <span className={`prompt-editor-pill-inner pill-${this.__kind}`}>
        <span className="prompt-editor-pill-kind">{pillKindLabel(this.__kind)}</span>
        <span>{this.__label}</span>
      </span>
    );
  }
}

export function $createPromptPillNode(pill: PromptContextPill): PromptPillNode {
  return new PromptPillNode(pill.id, pill.kind, pill.label, pill.value, pill.sourcePath, pill.relativePath);
}

export function $isPromptPillNode(node: LexicalNode | null | undefined): node is PromptPillNode {
  return node instanceof PromptPillNode;
}

export function $collectPromptPills(): PromptContextPill[] {
  const pills: PromptContextPill[] = [];

  function visit(node: LexicalNode) {
    if ($isPromptPillNode(node)) {
      pills.push(node.getPill());
      return;
    }
    const maybeParent = node as LexicalNode & { getChildren?: () => LexicalNode[] };
    if (typeof maybeParent.getChildren !== "function") return;
    for (const child of maybeParent.getChildren()) {
      visit(child);
    }
  }

  visit($getRoot());
  return pills;
}

export function createNodesFromPromptText(text: string): LexicalNode[] {
  const nodes: LexicalNode[] = [];
  const pattern = /(https?:\/\/[^\s"'<>]+)|(@(?:web|memory|draft|screenplay)\b)/gi;
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    if (match.index > lastIndex) {
      nodes.push($createTextNode(text.slice(lastIndex, match.index)));
    }
    const token = match[0];
    if (token.startsWith("http")) {
      nodes.push($createPromptPillNode({
        id: `url:${token}`,
        kind: "url",
        label: compactUrl(token),
        value: token,
      }));
    } else {
      nodes.push($createPromptPillNode({
        id: `tool:${token}`,
        kind: "tool",
        label: token,
        value: toolPromptValue(token),
      }));
    }
    lastIndex = pattern.lastIndex;
  }

  if (lastIndex < text.length) {
    nodes.push($createTextNode(text.slice(lastIndex)));
  }
  return nodes.length ? nodes : [$createTextNode(text)];
}

function convertPromptPillElement(domNode: HTMLElement): DOMConversionOutput | null {
  const id = domNode.getAttribute("data-pill-id");
  const kind = domNode.getAttribute("data-pill-kind") as PromptContextPillKind | null;
  const label = domNode.getAttribute("data-pill-label");
  const relativePath = domNode.getAttribute("data-pill-relative-path") ?? undefined;
  const sourcePath = domNode.getAttribute("data-pill-source-path") ?? undefined;
  const value = domNode.getAttribute("data-pill-value");
  if (!id || !kind || !label || value === null) return null;
  return { node: new PromptPillNode(id, kind, label, value, sourcePath, relativePath) };
}

function compactUrl(url: string) {
  try {
    const parsed = new URL(url);
    return parsed.hostname.replace(/^www\./, "");
  } catch {
    return "URL";
  }
}

function toolPromptValue(tool: string) {
  switch (tool) {
    case "@memory":
      return "启用写作记忆检索。";
    case "@draft":
      return "聚焦当前稿件。";
    case "@screenplay":
      return "使用短剧/剧本工具和约束。";
    default:
      return `启用工具 ${tool}。`;
  }
}

function pillKindLabel(kind: PromptContextPillKind) {
  switch (kind) {
    case "active-file":
      return "FILE";
    case "file":
      return "NOTE";
    case "image":
      return "IMG";
    case "memory":
      return "MEM";
    case "selection":
      return "SEL";
    case "tool":
      return "TOOL";
    case "url":
      return "URL";
  }
}
