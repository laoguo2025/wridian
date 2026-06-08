export type DraftReplaceEdit = {
  id: string;
  sourceRange?: { end: number; start: number };
  target: string;
};

export type DraftReplaceGuardReason =
  | "empty_target"
  | "target_not_found"
  | "target_ambiguous"
  | "overlap";

export type DraftReplaceMatch<TEdit extends DraftReplaceEdit> = {
  edit: TEdit;
  index: number;
};

export type DraftReplaceSkip<TEdit extends DraftReplaceEdit> = {
  edit: TEdit;
  reason: DraftReplaceGuardReason;
};

export type DraftReplaceGuardReport<TEdit extends DraftReplaceEdit> = {
  matches: DraftReplaceMatch<TEdit>[];
  skipped: DraftReplaceSkip<TEdit>[];
};

export function createDraftReplaceGuardReport<TEdit extends DraftReplaceEdit>(
  content: string,
  edits: TEdit[],
): DraftReplaceGuardReport<TEdit> {
  const claimed: Array<{ start: number; end: number }> = [];
  const matches: DraftReplaceMatch<TEdit>[] = [];
  const skipped: DraftReplaceSkip<TEdit>[] = [];

  for (const edit of edits) {
    const target = edit.target;
    if (!target) {
      skipped.push({ edit, reason: "empty_target" });
      continue;
    }

    if (edit.sourceRange) {
      const rangedMatch = matchSourceRange(content, edit);
      if (rangedMatch) {
        const range = { start: rangedMatch.index, end: rangedMatch.index + target.length };
        const overlaps = claimed.some((claimedRange) => range.start < claimedRange.end && range.end > claimedRange.start);
        if (overlaps) {
          skipped.push({ edit, reason: "overlap" });
          continue;
        }
        claimed.push(range);
        matches.push(rangedMatch);
        continue;
      }
    }

    const positions = findAllOccurrences(content, target);
    if (!positions.length) {
      skipped.push({ edit, reason: "target_not_found" });
      continue;
    }
    if (positions.length > 1) {
      skipped.push({ edit, reason: "target_ambiguous" });
      continue;
    }

    const index = positions[0];
    const end = index + target.length;
    const overlaps = claimed.some((range) => index < range.end && end > range.start);
    if (overlaps) {
      skipped.push({ edit, reason: "overlap" });
      continue;
    }

    claimed.push({ start: index, end });
    matches.push({ edit, index });
  }

  return {
    matches: matches.sort((left, right) => left.index - right.index),
    skipped,
  };
}

function matchSourceRange<TEdit extends DraftReplaceEdit>(
  content: string,
  edit: TEdit,
): DraftReplaceMatch<TEdit> | null {
  if (!edit.sourceRange) return null;
  const start = Math.max(0, Math.min(edit.sourceRange.start, content.length));
  const end = Math.max(start, Math.min(edit.sourceRange.end, content.length));
  if (content.slice(start, end) !== edit.target) {
    return null;
  }
  return { edit, index: start };
}

export function describeDraftReplaceSkip(reason: DraftReplaceGuardReason) {
  switch (reason) {
    case "empty_target":
      return "缺少原文片段";
    case "target_not_found":
      return "原文已变化";
    case "target_ambiguous":
      return "原文出现多次，需要重新定位";
    case "overlap":
      return "与其他修改范围重叠";
  }
}

function findAllOccurrences(content: string, target: string) {
  const positions: number[] = [];
  let index = content.indexOf(target);
  while (index >= 0) {
    positions.push(index);
    index = content.indexOf(target, index + Math.max(1, target.length));
  }
  return positions;
}
