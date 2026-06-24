// Presentation-only formatting for trace `subject` values. The raw `subject`
// remains the canonical identity (routing key, dedup key); this helper only
// derives a human-readable display label and never mutates the input.

export type SubjectDisplayKind = "internal" | "external" | "raw";

export interface SubjectDisplay {
  /** Classification used to decide rendering (badge vs plain). */
  kind: SubjectDisplayKind;
  /** The shortened, human-readable label to show. */
  label: string;
  /** True when the subject is a `file` path outside the repository root. */
  isExternal: boolean;
  /** The full, unmodified subject value (for tooltips / detail headings). */
  full: string;
}

const SEPARATORS = /[/\\]/;

/** Strip a trailing path separator so prefix comparison is exact. */
function normalizeRoot(repoRoot: string): string {
  return repoRoot.replace(/[/\\]+$/, "");
}

function lastTwoSegments(path: string): string {
  const segments = path.split(SEPARATORS).filter((segment) => segment.length > 0);
  return segments.slice(-2).join("/");
}

/**
 * Format a trace subject for display.
 *
 * - `file` subjects under `repoRoot` → repo-relative path (no leading separator).
 * - `file` subjects outside `repoRoot` → `isExternal`, label = last two segments.
 * - Non-`file` kinds, absent subjects, or an unknown repo root → raw pass-through.
 */
export function formatSubject(
  subject: string | null | undefined,
  repoRoot: string | null | undefined,
  subjectKind: string | null | undefined,
): SubjectDisplay {
  const full = subject ?? "";

  // Pass-through: non-file kind, absent subject, or repo root not yet resolved.
  if (subjectKind !== "file" || !subject || !repoRoot) {
    return { kind: "raw", label: full, isExternal: false, full };
  }

  const root = normalizeRoot(repoRoot);
  if (subject === root || subject.startsWith(`${root}/`) || subject.startsWith(`${root}\\`)) {
    const relative = subject.slice(root.length).replace(/^[/\\]+/, "");
    // Guard against an exact root match collapsing to an empty label.
    return { kind: "internal", label: relative || full, isExternal: false, full };
  }

  return { kind: "external", label: lastTwoSegments(subject), isExternal: true, full };
}
