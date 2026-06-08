export type DraftReplaceEdit = {
  id: string;
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
