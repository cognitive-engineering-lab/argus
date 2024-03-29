import {
  BodyHash,
  CharRange,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree,
} from "@argus/common/bindings";
import { CallArgus, ErrorJumpTargetInfo, Filename } from "@argus/common/lib";
import _ from "lodash";
import * as vscode from "vscode";

import { rangeHighlight } from "./decorate";
import { showErrorDialog } from "./errors";
import { globals } from "./main";
import { setup } from "./setup";
import {
  RustEditor,
  isDocumentInWorkspace,
  isRustDocument,
  isRustEditor,
  rustRangeToVscodeRange,
} from "./utils";
import { View } from "./view";

// NOTE: much of this file was inspired (or taken) from the rust-analyzer extension.
// See: https://github.com/rust-lang/rust-analyzer/blob/master/editors/code/src/ctx.ts#L1

export type Workspace =
  | { kind: "empty" }
  | { kind: "workspace-folder" }
  | { kind: "detached-files"; files: vscode.TextDocument[] };

export function fetchWorkspace(): Workspace {
  const folders = (vscode.workspace.workspaceFolders || []).filter(
    folder => folder.uri.scheme === "file"
  );
  const rustDocuments = vscode.workspace.textDocuments.filter(document =>
    isRustDocument(document)
  );

  return folders.length === 0
    ? rustDocuments.length === 0
      ? { kind: "empty" }
      : {
          kind: "detached-files",
          files: rustDocuments,
        }
    : { kind: "workspace-folder" };
}

function diagnosticMessage(type: "trait" | "ambig"): string {
  return type === "trait"
    ? "Expression contains unsatisfied trait bounds"
    : "Expression contains ambiguous types";
}

export interface Disposable {
  dispose(): void;
}

export type Cmd = (...args: any[]) => unknown;

export type CommandFactory = {
  enabled: (ctx: CtxInit) => Cmd;
  // disabled?: (ctx: Ctx) => Cmd;
};

// We can modify this if the initializations state changes.
export type CtxInit = Ctx;

export class Ctx {
  // Ranges used for highlighting code in the current Rust Editor.
  private highlightRanges: CharRange[] = [];
  private commandDisposables: Disposable[];
  private providingDisposables: Disposable[];
  private commandFactories: Record<string, CommandFactory>;
  private workspace: Workspace;
  private diagnosticCollection;
  private cache: BackendCache;
  public view: View | undefined;

  constructor(
    readonly extCtx: vscode.ExtensionContext,
    commandFactories: Record<string, CommandFactory>,
    workspace: Workspace
  ) {
    this.commandDisposables = [];
    this.providingDisposables = [];

    this.commandFactories = commandFactories;
    this.workspace = workspace;

    this.diagnosticCollection =
      vscode.languages.createDiagnosticCollection("rust");

    this.cache = new BackendCache(async () => {
      showErrorDialog(`
        Argus backend left uninitialized.
      `);
      return {
        type: "analysis-error",
        error: "Argus uninitialized.",
      };
    });
  }

  dispose() {
    _.forEach(this.commandDisposables, d => d.dispose());
    this.commandDisposables = [];

    _.forEach(this.providingDisposables, d => d.dispose());
    this.providingDisposables = [];
  }

  async createOrShowView(target?: ErrorJumpTargetInfo) {
    if (this.view) {
      this.view.getPanel().reveal();
    } else {
      const initialData = await this.getFreshWebViewData();
      this.view = new View(this.extCtx.extensionUri, initialData, target);
    }
  }

  get activeRustEditor(): RustEditor | undefined {
    const editor = vscode.window.activeTextEditor;
    return editor && isRustEditor(editor) ? editor : undefined;
  }

  get extensionPath(): string {
    return this.extCtx.extensionPath;
  }

  async setupBackend() {
    const b = await setup(this);
    if (b == null) {
      showErrorDialog("Failed to setup Argus");
      return;
    }
    // TODO: add some sort of "status loading" indicator.
    // Compile the workspace with the Argus version of rustc.
    await b(["preload"], true);
    this.cache = new BackendCache(b);

    await Promise.all(
      _.map(this.visibleEditors, editor => this.openEditor(editor))
    );

    // Register the commands with VSCode after the backend is setup.
    this.updateCommands();

    this.extCtx.subscriptions.push(this.diagnosticCollection);
    this.providingDisposables.push(
      vscode.languages.registerHoverProvider("rust", {
        provideHover: async (doc, pos, tok) => this.provideHover(doc, pos, tok),
      })
    );

    vscode.workspace.onDidChangeTextDocument(event => {
      const editor = vscode.window.activeTextEditor!;
      if (
        editor &&
        isRustEditor(editor) &&
        isDocumentInWorkspace(editor.document) &&
        event.document === editor.document &&
        editor.document.isDirty
      ) {
        globals.statusBar.setState("unsaved");
      }
    });

    vscode.window.onDidChangeActiveTextEditor(async editor => {
      if (
        editor &&
        isRustEditor(editor) &&
        isDocumentInWorkspace(editor.document)
      ) {
        console.debug(`Opening ${editor.document.fileName}`);
        this.openEditor(editor);
      }
    });

    vscode.workspace.onDidSaveTextDocument(async document => {
      const editor = vscode.window.activeTextEditor;
      if (
        editor &&
        isRustEditor(editor) &&
        editor.document === document &&
        isDocumentInWorkspace(editor.document)
      ) {
        this.cache.havoc();
        this.view!.reset(await this.getFreshWebViewData());
      }
    });
  }

  get visibleEditors(): RustEditor[] {
    return _.filter(vscode.window.visibleTextEditors, isRustEditor);
  }

  private async openEditor(editor: RustEditor) {
    // Load the obligations in the file, while we have the editor.
    const obl = await this.loadObligations(editor);
    if (obl) {
      if (this.view === undefined) {
        console.debug("View not initialized, skipping editor open.");
      }
      return this.view?.openEditor(editor, obl);
    }
  }

  private findVisibleEditorByName(name: Filename): RustEditor | undefined {
    return _.find(this.visibleEditors, e => e.document.fileName === name);
  }

  // Here we load the obligations for a file, and cache the results,
  // there's a distinction between having an editor and not because
  // we only have definitive access to the editor when it's visible.
  private async loadObligations(editor: RustEditor) {
    const obligations = await this.cache.getObligationsInBody(
      editor.document.fileName
    );
    if (obligations === undefined) {
      return;
    }

    this.refreshDiagnostics(editor, obligations);
    return obligations;
  }

  async getObligations(filename: Filename) {
    return this.cache.getObligationsInBody(filename);
  }

  async getTree(filename: Filename, obligation: Obligation, range: CharRange) {
    return this.cache.getTreeForObligation(filename, obligation, range);
  }

  // ------------------------------------
  // Diagnostic helpers

  private async provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    _token: vscode.CancellationToken
  ) {
    if (!isRustDocument(document) || !isDocumentInWorkspace(document)) {
      console.debug("Document not in workspace", document);
      return;
    }

    interface Rec {
      body: ObligationsInBody;
      bidx: BodyHash;
      eidx: ExprIdx;
      range: CharRange;
      hashes: ObligationHash[];
      type: "trait" | "ambig";
    }

    const info =
      (await this.cache.getObligationsInBody(document.fileName)) ?? [];

    const traitRecs: Rec[] = _.flatMap(info, ob =>
      _.map(ob.traitErrors, e => ({
        body: ob,
        bidx: ob.hash,
        eidx: e.idx,
        hashes: e.hashes,
        range: e.range,
        type: "trait",
      }))
    );
    const ambiRecs: Rec[] = _.flatMap(info, ob => {
      return _.map(ob.ambiguityErrors, e => ({
        body: ob,
        bidx: ob.hash,
        eidx: e.idx,
        hashes: [ob.obligations[ob.exprs[e.idx].obligations[0]].hash],
        range: e.range,
        type: "ambig",
      }));
    });
    const recs = _.concat(traitRecs, ambiRecs);
    const messages = _.map(recs, rec => {
      const range = rustRangeToVscodeRange(rec.range);

      if (!range.contains(position)) {
        console.debug("Skipping position outside of range", range, position);
        return;
      }

      return this.buildOpenErrorItemCmd(
        document.fileName,
        rec.bidx,
        rec.eidx,
        rec.hashes,
        rec.type
      );
    });

    return {
      contents: _.compact(messages),
    };
  }

  private refreshDiagnostics(editor: RustEditor, info: ObligationsInBody[]) {
    this.diagnosticCollection.clear();

    const traitDiags = _.flatMap(info, ob =>
      _.map(
        ob.traitErrors,
        e =>
          new vscode.Diagnostic(
            rustRangeToVscodeRange(e.range),
            diagnosticMessage("trait"),
            vscode.DiagnosticSeverity.Error
          )
      )
    );

    const ambigDiags = _.flatMap(info, ob =>
      _.map(
        ob.ambiguityErrors,
        e =>
          new vscode.Diagnostic(
            rustRangeToVscodeRange(e.range),
            diagnosticMessage("ambig"),
            vscode.DiagnosticSeverity.Error
          )
      )
    );

    this.diagnosticCollection.set(editor.document.uri, [
      ...traitDiags,
      ...ambigDiags,
    ]);
  }

  private buildOpenErrorItemCmd(
    filename: Filename,
    bodyIdx: BodyHash,
    exprIdx: ExprIdx,
    hashes: ObligationHash[],
    type: "trait" | "ambig"
  ): vscode.MarkdownString {
    const highlightUris = _.map(hashes, h =>
      vscode.Uri.parse(
        `command:argus.openError?${encodeURIComponent(
          JSON.stringify([filename, bodyIdx, exprIdx, h])
        )}`
      )
    );

    const msg = diagnosticMessage(type);
    const mdStr = new vscode.MarkdownString();
    mdStr.isTrusted = true;
    mdStr.appendText(msg + "\n");
    _.forEach(highlightUris, uri =>
      mdStr.appendMarkdown(`- [Open failure in argus debugger](${uri})\n`)
    );
    return mdStr;
  }

  async addHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findVisibleEditorByName(filename);
    if (editor) {
      this.highlightRanges.push(range);
      await this.refreshHighlights(editor);
    }
  }

  async removeHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findVisibleEditorByName(filename);
    if (editor) {
      this.highlightRanges = _.filter(
        this.highlightRanges,
        r => !_.isEqual(r, range)
      );
      await this.refreshHighlights(editor);
    }
  }

  private async refreshHighlights(editor: RustEditor) {
    editor.setDecorations(
      rangeHighlight,
      _.map(this.highlightRanges, r => rustRangeToVscodeRange(r))
    );
  }

  private async getFreshWebViewData(): Promise<
    [Filename, ObligationsInBody[]][]
  > {
    const initialData = await Promise.all(
      _.map(this.visibleEditors, async e => [
        e.document.fileName,
        await this.cache.getObligationsInBody(e.document.fileName),
      ])
    );
    // FIXME: how to make TS figure this out?
    return _.filter(initialData, ([_, obl]) => obl !== undefined) as any;
  }

  private updateCommands() {
    this.commandDisposables.forEach(disposable => disposable.dispose());
    this.commandDisposables = [];
    for (const [name, factory] of Object.entries(this.commandFactories)) {
      const fullName = `argus.${name}`;
      let callback = factory.enabled(this);
      this.commandDisposables.push(
        vscode.commands.registerCommand(fullName, callback)
      );
    }
  }
}

class BackendCache {
  private obligationCache: Record<
    Filename,
    Promise<ObligationsInBody[] | undefined>
  >;
  private treeCache: Record<
    Filename,
    Record<ObligationHash, Promise<SerializedTree | undefined>>
  >;
  private backend: CallArgus;

  constructor(backend: CallArgus) {
    this.obligationCache = {};
    this.treeCache = {};
    this.backend = backend;
  }

  havoc() {
    this.obligationCache = {};
    this.treeCache = {};
  }

  async getObligationsInBody(filename: Filename) {
    if (this.obligationCache[filename] !== undefined) {
      return this.obligationCache[filename];
    }

    const thunk = async () => {
      const res = await this.backend<"obligations">(["obligations", filename]);

      if (res.type !== "output") {
        globals.statusBar.setState("error");
        showErrorDialog(res.error);
        return;
      }
      return res.value;
    };

    this.obligationCache[filename] = thunk();
    return await this.obligationCache[filename];
  }

  async getTreeForObligation(
    filename: Filename,
    obl: Obligation,
    range: CharRange
  ) {
    if (this.treeCache[filename] !== undefined) {
      if (this.treeCache[filename][obl.hash] !== undefined) {
        return await this.treeCache[filename][obl.hash];
      }
    } else {
      this.treeCache[filename] = {};
    }

    const thunk = async () => {
      const res = await this.backend<"tree">([
        "tree",
        filename,
        obl.hash,
        range.start.line,
        range.start.column,
        range.end.line,
        range.end.column,
        obl.isSynthetic,
      ]);

      if (res.type !== "output") {
        globals.statusBar.setState("error");
        showErrorDialog(res.error);
        return;
      }

      // NOTE: the returned value should be an array of a single tree, however,
      // it is possible that no tree is returned. (Yes, but I'm working on it.)
      const tree0 = _.compact(res.value)[0];
      if (tree0 === undefined) {
        showErrorDialog(`
      Argus failed to find the appropriate proof tree.

      This is likely a bug in Argus, please report it.
      `);
        globals.statusBar.setState("error");
        return;
      }

      return tree0;
    };

    this.treeCache[filename][obl.hash] = thunk();
    return await this.treeCache[filename][obl.hash];
  }
}
