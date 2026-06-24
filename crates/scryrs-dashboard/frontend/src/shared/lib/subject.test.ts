import { describe, expect, it } from "vitest";
import { formatSubject } from "@/shared/lib/subject";

const ROOT = "/Users/me/repos/scryrs";

describe("formatSubject", () => {
  it("strips the repository root for in-repo files", () => {
    const result = formatSubject(`${ROOT}/.devagent/doc_build/architecture.md`, ROOT, "file");
    expect(result).toEqual({
      kind: "internal",
      label: ".devagent/doc_build/architecture.md",
      isExternal: false,
      full: `${ROOT}/.devagent/doc_build/architecture.md`,
    });
  });

  it("tolerates a trailing separator on the repository root", () => {
    const result = formatSubject(`${ROOT}/src/main.rs`, `${ROOT}/`, "file");
    expect(result.kind).toBe("internal");
    expect(result.label).toBe("src/main.rs");
  });

  it("badges external files with the last two path segments", () => {
    const subject =
      "/Users/me/repos/dignitas/cl-sessions/dignitas-agentic-docs/openspec/changes/agentic-docs-presentation/proposal.md";
    const result = formatSubject(subject, ROOT, "file");
    expect(result).toEqual({
      kind: "external",
      label: "agentic-docs-presentation/proposal.md",
      isExternal: true,
      full: subject,
    });
  });

  it("handles an external path with a single segment gracefully", () => {
    const result = formatSubject("/proposal.md", ROOT, "file");
    expect(result.kind).toBe("external");
    expect(result.isExternal).toBe(true);
    expect(result.label).toBe("proposal.md");
  });

  it("passes non-file subjects through unchanged", () => {
    const result = formatSubject("routing", ROOT, "routing");
    expect(result).toEqual({ kind: "raw", label: "routing", isExternal: false, full: "routing" });
  });

  it("passes an absent subject through as raw", () => {
    const result = formatSubject(null, ROOT, "file");
    expect(result).toEqual({ kind: "raw", label: "", isExternal: false, full: "" });
  });

  it("passes the raw value through when the repository root is unknown", () => {
    const subject = `${ROOT}/src/main.rs`;
    const result = formatSubject(subject, null, "file");
    expect(result).toEqual({ kind: "raw", label: subject, isExternal: false, full: subject });
  });
});
