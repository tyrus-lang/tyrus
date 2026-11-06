// Command arguments from the language server are protocol JSON, not VS Code
// objects. Keep their validation independent from the VS Code API so both
// `changes` and `documentChanges` stay directly testable.
export interface LspPosition {
  line: number;
  character: number;
}

export interface LspTextEdit {
  range: { start: LspPosition; end: LspPosition };
  newText: string;
}

export interface LspTextEditsForUri {
  uri: string;
  edits: LspTextEdit[];
}

function isLspPosition(value: unknown): value is LspPosition {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<LspPosition>;
  return (
    typeof candidate.line === "number" &&
    typeof candidate.character === "number"
  );
}

function isLspTextEdit(value: unknown): value is LspTextEdit {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<LspTextEdit>;
  return (
    typeof candidate.newText === "string" &&
    !!candidate.range &&
    isLspPosition(candidate.range.start) &&
    isLspPosition(candidate.range.end)
  );
}

function textEdits(value: unknown): LspTextEdit[] | undefined {
  if (!Array.isArray(value)) {
    return undefined;
  }
  return value.filter(isLspTextEdit);
}

// Extracts only text-document edits. The server currently emits no resource
// operations, and ignoring an unknown document-change variant is safer than
// attempting an incomplete create, rename, or delete operation.
export function lspTextEditsByUri(value: unknown): LspTextEditsForUri[] | undefined {
  if (!value || typeof value !== "object") {
    return undefined;
  }
  const raw = value as {
    changes?: unknown;
    documentChanges?: unknown;
  };
  const result: LspTextEditsForUri[] = [];
  if (raw.changes && typeof raw.changes === "object") {
    for (const [uri, edits] of Object.entries(raw.changes)) {
      const parsedEdits = textEdits(edits);
      if (parsedEdits) {
        result.push({ uri, edits: parsedEdits });
      }
    }
  }
  if (Array.isArray(raw.documentChanges)) {
    for (const change of raw.documentChanges) {
      if (!change || typeof change !== "object") {
        continue;
      }
      const candidate = change as {
        textDocument?: { uri?: unknown };
        edits?: unknown;
      };
      const parsedEdits = textEdits(candidate.edits);
      if (typeof candidate.textDocument?.uri === "string" && parsedEdits) {
        result.push({ uri: candidate.textDocument.uri, edits: parsedEdits });
      }
    }
  }
  return result;
}
