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
  EditorState,
  KEY_ENTER_COMMAND,
  TextNode,
} from "lexical";
import type { PromptSuggestion } from "./messageRepository";

export function CopilotPromptEditor({
  onChange,
  onSelectSuggestion,
  onSubmit,
  placeholder,
  suggestions,
  value,
}: {
  onChange: (value: string) => void;
  onSelectSuggestion: (suggestion: PromptSuggestion) => void;
  onSubmit: () => void;
  placeholder: string;
  suggestions: PromptSuggestion[];
  value: string;
}) {
  const initialConfig = useMemo(
    () => ({
      namespace: "WridianChatInput",
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
          contentEditable={<ContentEditable className="prompt-editor" aria-label="共创输入" />}
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
      replacePromptTrigger(currentTrigger, suggestion.kind === "command" ? suggestion.insertText : "");
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

function replacePromptTrigger(trigger: PromptTriggerState, replacement: string) {
  const selection = $getSelection();
  if (!$isRangeSelection(selection)) return;
  const anchor = selection.anchor;
  const anchorNode = anchor.getNode();
  if (!(anchorNode instanceof TextNode)) return;
  const text = anchorNode.getTextContent();
  const normalizedReplacement = replacement ? `${replacement}${replacement.endsWith(" ") ? "" : " "}` : "";
  const nextText = `${text.slice(0, trigger.start)}${normalizedReplacement}${text.slice(trigger.end)}`;
  const cursor = trigger.start + normalizedReplacement.length;
  anchorNode.setTextContent(nextText);
  anchorNode.select(cursor, cursor);
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
        paragraph.append($createTextNode(value));
      }
      root.append(paragraph);
    });
  }, [editor, value]);

  return null;
}
