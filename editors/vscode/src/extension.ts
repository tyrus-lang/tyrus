import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import { lspTextEditsByUri, type LspTextEdit } from "./lsp-workspace-edit";

let client: LanguageClient | undefined;

const errorLineDecoration = vscode.window.createTextEditorDecorationType({
  isWholeLine: true,
  backgroundColor: "rgba(255, 64, 64, 0.14)",
  overviewRulerColor: "rgba(255, 64, 64, 0.85)",
  overviewRulerLane: vscode.OverviewRulerLane.Full,
});

const warningLineDecoration = vscode.window.createTextEditorDecorationType({
  isWholeLine: true,
  backgroundColor: "rgba(255, 190, 0, 0.12)",
  overviewRulerColor: "rgba(255, 190, 0, 0.85)",
  overviewRulerLane: vscode.OverviewRulerLane.Full,
});

function supportsDiagnosticDecoration(document: vscode.TextDocument): boolean {
  return (
    document.languageId === "tyrus" ||
    document.languageId === "tyrus-env" ||
    document.languageId === "dotenv"
  );
}

function updateDiagnosticDecorations(editor: vscode.TextEditor): void {
  const enabled = vscode.workspace
    .getConfiguration("tyrus", editor.document.uri)
    .get<boolean>("diagnostics.highlightWholeLine", true);
  if (!enabled || !supportsDiagnosticDecoration(editor.document)) {
    editor.setDecorations(errorLineDecoration, []);
    editor.setDecorations(warningLineDecoration, []);
    return;
  }

  const errorLines = new Set<number>();
  const warningLines = new Set<number>();
  for (const diagnostic of vscode.languages.getDiagnostics(editor.document.uri)) {
    const line = Math.min(
      diagnostic.range.start.line,
      Math.max(editor.document.lineCount - 1, 0),
    );
    if (diagnostic.severity === vscode.DiagnosticSeverity.Error) {
      errorLines.add(line);
      warningLines.delete(line);
    } else if (
      diagnostic.severity === vscode.DiagnosticSeverity.Warning &&
      !errorLines.has(line)
    ) {
      warningLines.add(line);
    }
  }

  const ranges = (lines: Set<number>): vscode.Range[] =>
    [...lines]
      .sort((left, right) => left - right)
      .map((line) => editor.document.lineAt(line).range);
  editor.setDecorations(errorLineDecoration, ranges(errorLines));
  editor.setDecorations(warningLineDecoration, ranges(warningLines));
}

function updateVisibleDiagnosticDecorations(): void {
  for (const editor of vscode.window.visibleTextEditors) {
    updateDiagnosticDecorations(editor);
  }
}

// Maps an LSP TextEdit onto the document. A full-range replace covers plain
// edits and pure insertions (empty range) alike.
function applyLspTextEdits(
  edit: vscode.WorkspaceEdit,
  uri: vscode.Uri,
  textEdits: readonly LspTextEdit[],
): void {
  for (const change of textEdits) {
    edit.replace(
      uri,
      new vscode.Range(
        new vscode.Position(change.range.start.line, change.range.start.character),
        new vscode.Position(change.range.end.line, change.range.end.character),
      ),
      change.newText,
    );
  }
}

function workspaceEditFromLsp(value: unknown): vscode.WorkspaceEdit | undefined {
  const changes = lspTextEditsByUri(value);
  if (!changes) {
    return undefined;
  }
  const edit = new vscode.WorkspaceEdit();
  for (const change of changes) {
    applyLspTextEdits(edit, vscode.Uri.parse(change.uri), change.edits);
  }
  return edit;
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const configuration = vscode.workspace.getConfiguration("tyrus");
  const command = configuration.get<string>("server.path", "tyrus");
  const serverOptions: ServerOptions = {
    command,
    args: ["lsp"],
    options: {
      cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
    },
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "tyrus" },
      // Env files use their own language for highlighting; the server also
      // provides typed diagnostics and `@` decorator completion for them.
      { scheme: "file", language: "tyrus-env" },
      // VS Code's dotenv language association can take precedence over the
      // Tyrus association for `.env` files, so attach to it as well.
      { scheme: "file", language: "dotenv" },
    ],
    // The server advertises document formatting (via tyrus-fmt) only when this
    // is enabled, so `tyrus.format.enable` keeps working through the LSP.
    // The same applies to Quick Fix code lenses behind
    // `tyrus.codeLens.quickFixes.enable`.
    initializationOptions: {
      formatEnable: configuration.get<boolean>("format.enable", true),
      codeLensQuickFixesEnable: configuration.get<boolean>(
        "codeLens.quickFixes.enable",
        true,
      ),
      workspaceMaxFiles: configuration.get<number>("workspace.maxFiles", 10_000),
      workspaceMaxDepth: configuration.get<number>("workspace.maxDepth", 64),
      workspaceExcludeDirectories: configuration.get<string[]>(
        "workspace.excludeDirectories",
        [],
      ),
    },
    outputChannelName: "tyrus Language Server",
  };

  client = new LanguageClient(
    "tyrusLanguageServer",
    "tyrus Language Server",
    serverOptions,
    clientOptions,
  );
  context.subscriptions.push(
    client,
    errorLineDecoration,
    warningLineDecoration,
    vscode.languages.onDidChangeDiagnostics((event) => {
      const changed = new Set(event.uris.map((uri) => uri.toString()));
      for (const editor of vscode.window.visibleTextEditors) {
        if (changed.has(editor.document.uri.toString())) {
          updateDiagnosticDecorations(editor);
        }
      }
    }),
    vscode.window.onDidChangeVisibleTextEditors(() => {
      updateVisibleDiagnosticDecorations();
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("tyrus.diagnostics.highlightWholeLine")) {
        updateVisibleDiagnosticDecorations();
      }
    }),
    // Applies the edit carried by a Quick Fix code lens. The argument arrives
    // as protocol JSON from `textDocument/codeLens`, never as a revived
    // vscode.WorkspaceEdit, hence the explicit conversion.
    vscode.commands.registerCommand("tyrus.applyFix", async (edit?: unknown) => {
      const workspaceEdit = workspaceEditFromLsp(edit);
      if (!workspaceEdit) {
        return;
      }
      await vscode.workspace.applyEdit(workspaceEdit);
    }),
    vscode.commands.registerCommand(
      "tyrus.restartLanguageServer",
      async () => {
        if (!client) {
          void vscode.window.showErrorMessage(
            "The tyrus Language Server is not running.",
          );
          return;
        }
        try {
          await client.restart();
          void vscode.window.showInformationMessage(
            "The tyrus Language Server was restarted.",
          );
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          void vscode.window.showErrorMessage(
            `Could not restart the tyrus Language Server: ${message}`,
          );
        }
      },
    ),
  );
  try {
    await client.start();
  } catch (error) {
    client = undefined;
    const message = error instanceof Error ? error.message : String(error);
    void vscode.window.showErrorMessage(
      `Could not start the tyrus Language Server: ${message}. ` +
        "Check tyrus.server.path.",
    );
  }
  updateVisibleDiagnosticDecorations();
  // Formatting, diagnostics, hover, and completion are all provided by the
  // language server; the extension stays a thin client with no language logic.
}

export async function deactivate(): Promise<void> {
  await client?.stop();
  client = undefined;
}
