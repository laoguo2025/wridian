import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { LexicalComposer } from "@lexical/react/LexicalComposer";
import { PlainTextPlugin } from "@lexical/react/LexicalPlainTextPlugin";
import { ContentEditable } from "@lexical/react/LexicalContentEditable";
import { HistoryPlugin } from "@lexical/react/LexicalHistoryPlugin";
import { OnChangePlugin } from "@lexical/react/LexicalOnChangePlugin";
import { LexicalErrorBoundary } from "@lexical/react/LexicalErrorBoundary";
import { useLexicalComposerContext } from "@lexical/react/LexicalComposerContext";
import {
  $createParagraphNode,
  $createTextNode,
  $getSelection,
  $getRoot,
  $isRangeSelection,
  COMMAND_PRIORITY_LOW,
  COMMAND_PRIORITY_CRITICAL,
  DELETE_CHARACTER_COMMAND,
  EditorState,
  KEY_ENTER_COMMAND,
  PASTE_COMMAND,
  TextNode,
  type LexicalNode,
} from "lexical";
import { createPromptPillFromSuggestion, type PromptContextPill, type PromptSuggestion } from "./promptContext";
import { $collectPromptPills, $createPromptPillNode, $isPromptPillNode, createNodesFromPromptText, PromptPillNode } from "./promptPillNodes";

export function CopilotPromptEditor({
  onChange,
  onImagePaste,
  onPillsChange,
  onSelectSuggestion,
  onSubmit,
  placeholder,
  suggestions,
  value,
}: {
  onChange: (value: string) => void;
  onImagePaste?: (files: File[]) => void;
  onPillsChange: (pills: PromptContextPill[]) => void;
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  onSubmit: () => void;
  placeholder: string;
  suggestions: PromptSuggestion[];
  value: string;
}) {
  const initialConfig = useMemo(
    () => ({
      namespace: "WridianChatInput",
      nodes: [PromptPillNode],
      theme: {
        paragraph: "prompt-editor-paragraph",
        root: "prompt-editor-root",
      },
      onError(error: Error) {
        console.error("Wridian chat input error", error);
      },
    }),
    [],
  );

  return (
    <LexicalComposer initialConfig={initialConfig}>
      <div className="prompt-editor-shell">
        <PlainTextPlugin
          contentEditable={<ContentEditable className="prompt-editor" aria-label="对话输入" />}
          placeholder={<div className="prompt-placeholder">{placeholder}</div>}
          ErrorBoundary={LexicalErrorBoundary}
        />
        <OnChangePlugin
          onChange={(editorState: EditorState) => {
            editorState.read(() => {
              onChange($getRoot().getTextContent());
            });
          }}
        />
        <HistoryPlugin />
        <PromptKeyboardPlugin onSubmit={onSubmit} />
        <PromptPillSyncPlugin onPillsChange={onPillsChange} />
        <PromptPillDeletionPlugin />
        <PromptPastePlugin onImagePaste={onImagePaste} />
        <PromptTypeaheadPlugin onSelectSuggestion={onSelectSuggestion} suggestions={suggestions} />
        <PromptValueSyncPlugin value={value} />
      </div>
    </LexicalComposer>
  );
}

function PromptKeyboardPlugin({ onSubmit }: { onSubmit: () => void }) {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    return editor.registerCommand(
      KEY_ENTER_COMMAND,
      (event: KeyboardEvent | null) => {
        if (!event) return false;
        if (event.isComposing || event.key === "Process") {
          event.preventDefault();
          return true;
        }
        if (!event.shiftKey && !event.metaKey && !event.ctrlKey && !event.altKey) {
          event.preventDefault();
          onSubmit();
          return true;
        }
        return false;
      },
      COMMAND_PRIORITY_LOW,
    );
  }, [editor, onSubmit]);

  return null;
}

type PromptTriggerState = {
  end: number;
  query: string;
  start: number;
  trigger: "@" | "/";
};

function PromptTypeaheadPlugin({
  onSelectSuggestion,
  suggestions,
}: {
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  suggestions: PromptSuggestion[];
}) {
  const [editor] = useLexicalComposerContext();
  const [triggerState, setTriggerState] = useState<PromptTriggerState | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const triggerStateRef = useRef<PromptTriggerState | null>(null);
  const filteredSuggestionsRef = useRef<PromptSuggestion[]>([]);
  const selectedIndexRef = useRef(0);
  const selectSuggestionRef = useRef<(suggestion: PromptSuggestion) => void>(() => {});
  const filteredSuggestions = useMemo(() => {
    if (!triggerState) return [];
    const kind = triggerState.trigger === "@" ? "context" : "command";
    const query = triggerState.query.trim().toLowerCase();
    return suggestions
      .filter((suggestion) => suggestion.kind === kind)
      .filter((suggestion) => {
        if (!query) return true;
        return `${suggestion.label} ${suggestion.detail}`.toLowerCase().includes(query);
      })
      .slice(0, 7);
  }, [suggestions, triggerState]);

  useEffect(() => {
    triggerStateRef.current = triggerState;
  }, [triggerState]);

  useEffect(() => {
    filteredSuggestionsRef.current = filteredSuggestions;
  }, [filteredSuggestions]);

  useEffect(() => {
    selectedIndexRef.current = selectedIndex;
  }, [selectedIndex]);

  useEffect(() => {
    if (selectedIndex >= filteredSuggestions.length) {
      setSelectedIndex(0);
    }
  }, [filteredSuggestions.length, selectedIndex]);

  const closeMenu = useCallback(() => {
    setTriggerState(null);
    setSelectedIndex(0);
  }, []);

  const selectSuggestion = useCallback((suggestion: PromptSuggestion) => {
    const currentTrigger = triggerStateRef.current;
    if (!currentTrigger) return;
    editor.update(() => {
      replacePromptTrigger(currentTrigger, suggestion);
    });
    if (suggestion.kind === "context") {
      onSelectSuggestion(suggestion);
    }
    closeMenu();
  }, [closeMenu, editor, onSelectSuggestion]);

  useEffect(() => {
    selectSuggestionRef.current = selectSuggestion;
  }, [selectSuggestion]);

  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const selection = $getSelection();
        if (!$isRangeSelection(selection) || !selection.isCollapsed()) {
          setTriggerState(null);
          return;
        }

        const anchor = selection.anchor;
        const anchorNode = anchor.getNode();
        if (!(anchorNode instanceof TextNode)) {
          setTriggerState(null);
          return;
        }

        setTriggerState(readPromptTrigger(anchorNode.getTextContent(), anchor.offset));
      });
    });
  }, [editor]);

  useEffect(() => {
    let currentRoot: HTMLElement | null = null;
    const handleKeyDown = (event: KeyboardEvent) => {
      const currentSuggestions = filteredSuggestionsRef.current;
      if (!triggerStateRef.current || !currentSuggestions.length) return;
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setSelectedIndex((current) => (current + 1) % currentSuggestions.length);
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        setSelectedIndex((current) => (current - 1 + currentSuggestions.length) % currentSuggestions.length);
      } else if (event.key === "Escape") {
        event.preventDefault();
        closeMenu();
      } else if (event.key === "Tab" || event.key === "Enter") {
        event.preventDefault();
        selectSuggestionRef.current(currentSuggestions[selectedIndexRef.current] ?? currentSuggestions[0]);
      }
    };
    const unregister = editor.registerRootListener((rootElement, prevRootElement) => {
      if (prevRootElement) {
        prevRootElement.removeEventListener("keydown", handleKeyDown, true);
      }
      if (rootElement) {
        rootElement.addEventListener("keydown", handleKeyDown, true);
      }
      currentRoot = rootElement;
    });
    return () => {
      currentRoot?.removeEventListener("keydown", handleKeyDown, true);
      unregister();
    };
  }, [closeMenu, editor]);

  if (!triggerState || !filteredSuggestions.length) return null;

  return (
    <div className="prompt-suggestion-menu" role="listbox">
      {filteredSuggestions.map((suggestion, index) => (
        <button
          type="button"
          className={`prompt-suggestion-item ${index === selectedIndex ? "selected" : ""}`}
          key={suggestion.id}
          onMouseDown={(event) => {
            event.preventDefault();
            selectSuggestion(suggestion);
          }}
          role="option"
          aria-selected={index === selectedIndex}
        >
          <span>{suggestion.label}</span>
          <small>{suggestion.detail}</small>
        </button>
      ))}
    </div>
  );
}

function readPromptTrigger(text: string, offset: number): PromptTriggerState | null {
  const textBeforeCaret = text.slice(0, offset);
  const match = /(^|\s)([@/])([^\n@/]*)$/.exec(textBeforeCaret);
  if (!match) return null;
  const query = match[3] ?? "";
  if (query.length > 36) return null;
  return {
    end: offset,
    query,
    start: match.index + (match[1]?.length ?? 0),
    trigger: match[2] as "@" | "/",
  };
}

function replacePromptTrigger(trigger: PromptTriggerState, suggestion: PromptSuggestion) {
  const selection = $getSelection();
  if (!$isRangeSelection(selection)) return;
  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();
  if (!(anchorNode instanceof TextNode)) return;
  const text = anchorNode.getTextContent();
  const beforeText = text.slice(0, trigger.start);
  const afterText = text.slice(trigger.end);
  const nodes: LexicalNode[] = [];
  if (beforeText) nodes.push($createTextNode(beforeText));
  if (suggestion.kind === "context") {
    nodes.push($createPromptPillNode(createPromptPillFromSuggestion(suggestion)));
    nodes.push($createTextNode(afterText ? ` ${afterText}` : " "));
  } else {
    nodes.push(...createNodesFromPromptText(`${suggestion.insertText}${suggestion.insertText.endsWith(" ") ? "" : " "}`));
    if (afterText) nodes.push($createTextNode(afterText));
  }
  replaceTextNodeWithNodes(anchorNode, nodes);
}

function PromptValueSyncPlugin({ value }: { value: string }) {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    editor.update(() => {
      const root = $getRoot();
      if (root.getTextContent() === value) return;
      root.clear();
      const paragraph = $createParagraphNode();
      if (value) {
        paragraph.append(...createNodesFromPromptText(value));
      }
      root.append(paragraph);
    });
  }, [editor, value]);

  return null;
}

function PromptPillSyncPlugin({ onPillsChange }: { onPillsChange: (pills: PromptContextPill[]) => void }) {
  const [editor] = useLexicalComposerContext();
  const previousRef = useRef("");

  useEffect(() => {
    return editor.registerUpdateListener(({ editorState }) => {
      editorState.read(() => {
        const pills = $collectPromptPills();
        const serialized = JSON.stringify(pills);
        if (serialized === previousRef.current) return;
        previousRef.current = serialized;
        onPillsChange(pills);
      });
    });
  }, [editor, onPillsChange]);

  return null;
}

function PromptPillDeletionPlugin() {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    return editor.registerCommand(
      DELETE_CHARACTER_COMMAND,
      (isBackward: boolean) => {
        let handled = false;
        editor.update(() => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection) || !selection.isCollapsed()) return;
          const anchorNode = selection.anchor.getNode();
          if ($isPromptPillNode(anchorNode)) {
            anchorNode.remove();
            handled = true;
            return;
          }
          if (!isBackward || selection.anchor.offset !== 0) return;
          const previous = anchorNode.getPreviousSibling();
          if ($isPromptPillNode(previous)) {
            previous.remove();
            handled = true;
          }
        });
        return handled;
      },
      COMMAND_PRIORITY_CRITICAL,
    );
  }, [editor]);

  return null;
}

function PromptPastePlugin({ onImagePaste }: { onImagePaste?: (files: File[]) => void }) {
  const [editor] = useLexicalComposerContext();

  useEffect(() => {
    return editor.registerCommand(
      PASTE_COMMAND,
      (event: ClipboardEvent) => {
        const data = event.clipboardData;
        if (!data) return false;
        const imageFiles = Array.from(data.items ?? [])
          .filter((item) => item.type.startsWith("image/"))
          .map((item) => item.getAsFile())
          .filter((file): file is File => Boolean(file));
        if (imageFiles.length && onImagePaste) {
          event.preventDefault();
          onImagePaste(imageFiles);
          return true;
        }

        const text = data.getData("text/plain");
        if (!text || (!text.includes("http") && !/@(?:web|memory|draft|screenplay)\b/i.test(text))) {
          return false;
        }
        event.preventDefault();
        editor.update(() => {
          const selection = $getSelection();
          if (!$isRangeSelection(selection)) return;
          selection.insertNodes(createNodesFromPromptText(text));
        });
        return true;
      },
      COMMAND_PRIORITY_CRITICAL,
    );
  }, [editor, onImagePaste]);

  return null;
}

function replaceTextNodeWithNodes(textNode: TextNode, nodes: LexicalNode[]) {
  nodes.forEach((node, index) => {
    if (index === 0) {
      textNode.replace(node);
    } else {
      nodes[index - 1].insertAfter(node);
    }
  });
  const lastNode = nodes[nodes.length - 1];
  if (lastNode) {
    if (lastNode instanceof TextNode) {
      const length = lastNode.getTextContent().length;
      lastNode.select(length, length);
    } else {
      lastNode.selectNext();
    }
  }
}
