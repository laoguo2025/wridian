import {
  type KeyboardEvent as ReactKeyboardEvent,
  type RefObject,
  useEffect,
} from "react";
import type { ChatDraftEdit } from "../chat/chatManager";
import { createDraftReplaceGuardReport } from "./draftReplaceGuard";

type DraftEdit = ChatDraftEdit;

export type TextSelection = {
  start: number;
  end: number;
};

export type AppliedDraftEdit = {
  end: number;
  id: string;
  replacement: string;
  start: number;
  target: string;
};

export function DraftEditor({
  appliedEdits,
  content,
  editorRef,
  edits,
  onAcceptEdit,
  onChange,
  onKeyDown,
  onRejectEdit,
  onSelectionActionDismiss,
  onSelectionChange,
}: {
  appliedEdits?: AppliedDraftEdit[];
  content: string;
  editorRef: RefObject<HTMLDivElement | null>;
  edits: DraftEdit[];
  onAcceptEdit: (id: string) => void;
  onChange: (content: string) => void;
  onKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void;
  onRejectEdit: (id: string) => void;
  onSelectionActionDismiss?: () => void;
  onSelectionChange: () => void;
}) {
  const chunks = buildDraftEditorChunks(content, edits, appliedEdits ?? []);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || edits.length) return;
    if (!(appliedEdits ?? []).length && editor.innerText !== content) {
      editor.innerText = content;
    }
  }, [appliedEdits, content, editorRef, edits.length]);

  return (
    <div
      ref={editorRef}
      className="draft-editor"
      contentEditable={!edits.length}
      role="textbox"
      aria-label="正文"
      spellCheck={false}
      suppressContentEditableWarning
      onInput={(event) => {
        onSelectionActionDismiss?.();
        onChange(event.currentTarget.innerText);
      }}
      onKeyDown={onKeyDown}
      onKeyUp={onSelectionChange}
      onMouseUp={onSelectionChange}
      onScroll={onSelectionActionDismiss}
    >
      {chunks.map((chunk, index) => {
        if (chunk.kind === "text") {
          return <span key={`text-${index}`}>{chunk.text}</span>;
        }
        if (chunk.kind === "applied") {
          return (
            <span
              className="inline-applied-edit"
              key={chunk.edit.id}
              title={`原文：${chunk.edit.target}`}
            >
              {chunk.text}
            </span>
          );
        }
        return (
          <span className="inline-edit" key={chunk.edit.id}>
            <span className="inline-diff">
              {chunk.edit.target ? <del>{chunk.edit.target}</del> : null}
              <ins>{chunk.edit.replacement}</ins>
            </span>
            <span className="inline-edit-actions" contentEditable={false}>
              <button type="button" title="确认后写入正文" onClick={() => onAcceptEdit(chunk.edit.id)}>确认</button>
              <button type="button" className="secondary" title="取消这处建议，正文保持原样" onClick={() => onRejectEdit(chunk.edit.id)}>取消</button>
            </span>
          </span>
        );
      })}
    </div>
  );
}

export function readContentEditableSelection(root: HTMLElement): TextSelection | null {
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

export function setContentEditableCaret(root: HTMLElement | null, offset: number) {
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
  | { kind: "applied"; edit: AppliedDraftEdit; text: string }
  | { kind: "edit"; edit: DraftEdit };

function buildDraftEditorChunks(
  content: string,
  edits: DraftEdit[],
  appliedEdits: AppliedDraftEdit[],
): DraftSuggestionChunk[] {
  if (!edits.length && appliedEdits.length) {
    return buildAppliedEditChunks(content, appliedEdits);
  }
  return buildDraftSuggestionChunks(content, edits);
}

function buildAppliedEditChunks(content: string, appliedEdits: AppliedDraftEdit[]): DraftSuggestionChunk[] {
  const chunks: DraftSuggestionChunk[] = [];
  let cursor = 0;
  for (const edit of [...appliedEdits].sort((left, right) => left.start - right.start)) {
    const start = Math.max(0, Math.min(edit.start, content.length));
    const end = Math.max(start, Math.min(edit.end, content.length));
    if (start < cursor || end <= start) continue;
    if (content.slice(start, end) !== edit.replacement) continue;
    if (start > cursor) {
      chunks.push({ kind: "text", text: content.slice(cursor, start) });
    }
    chunks.push({ kind: "applied", edit, text: content.slice(start, end) });
    cursor = end;
  }
  if (cursor < content.length) {
    chunks.push({ kind: "text", text: content.slice(cursor) });
  }
  return chunks.length ? chunks : [{ kind: "text", text: content }];
}

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
