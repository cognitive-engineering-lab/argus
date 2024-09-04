import BodyInfo from "@argus/common/BodyInfo";
import type {
  BodyHash,
  CharRange,
  DefLocation,
  ExprIdx,
  Obligation,
  ObligationHash,
  ObligationsInBody,
  SerializedTree
} from "@argus/common/bindings";
import { makeid } from "@argus/common/func";
import type {
  CallArgus,
  ErrorJumpTargetInfo,
  FileInfo,
  Filename
} from "@argus/common/lib";
import type { CancelablePromise as CPromise } from "cancelable-promise";
import _ from "lodash";
import * as vscode from "vscode";

import { rangeHighlight } from "./decorate";
import { showErrorDialog } from "./errors";
import { log } from "./logging";
import { StatusBar } from "./statusbar";
import {
  type RustEditor,
  isDocumentInWorkspace,
  isRustDocument,
  isRustEditor,
  rustRangeToVscodeRange
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
          files: rustDocuments
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
  enabled: (ctx: Ctx) => Cmd;
  // disabled?: (ctx: Ctx) => Cmd;
};

/**
 * Extension context before a system backend has been set.
 */
export class CtxInit {
  readonly statusBar: StatusBar;

  constructor(
    readonly extCtx: vscode.ExtensionContext,
    readonly workspace: Workspace & { kind: "workspace-folder" }
  ) {
    this.statusBar = new StatusBar(extCtx);
  }

  dispose() {
    this.statusBar.dispose();
  }
}

export class Ctx {
  private view?: View;
  private highlightRanges: CharRange[] = [];
  private commandDisposables: Disposable[] = [];
  private disposables: Disposable[] = [];

  private constructor(
    readonly ictxt: CtxInit,
    readonly cache: BackendCache,
    readonly commandFactories: Record<string, CommandFactory>,

    // Ranges used for highlighting code in the current Rust Editor.
    readonly diagnosticCollection: vscode.DiagnosticCollection
  ) {}

  static async make(
    initial: CtxInit,
    backend: CallArgus,
    commandFactories: Record<string, CommandFactory>
  ) {
    await backend(
      ["preload"],
      true, // Don't expect output
      true // Ignore a failing exit code
    );

    const self = new Ctx(
      initial,
      new BackendCache(initial.statusBar, backend),
      commandFactories,
      vscode.languages.createDiagnosticCollection("rust")
    );

    // Preload information for visible editors.
    if (self.activeRustEditor) {
      self.visibleEditors.forEach(e => self.openEditor(e));
    }

    log("Caching open editor information");

    // Register the commands with VSCode after the backend is setup.
    // NOTE: nothing should be registered before the commands...
    self.updateCommands();

    log("Updated commands");
    self.registerSubscriptionsAndDisposables();

    return self;
  }

  private registerSubscriptionsAndDisposables() {
    this.ictxt.extCtx.subscriptions.push(this.diagnosticCollection!);
    this.disposables.push(
      vscode.languages.registerHoverProvider("rust", {
        provideHover: async (doc, pos, tok) => this.provideHover(doc, pos, tok)
      })
    );

    this.disposables.push(
      vscode.workspace.onDidChangeTextDocument(event => {
        const editor = vscode.window.activeTextEditor!;
        if (
          editor &&
          isRustEditor(editor) &&
          isDocumentInWorkspace(editor.document) &&
          event.document === editor.document &&
          editor.document.isDirty
        ) {
          this.ictxt.statusBar.setState("unsaved");
        }
      })
    );

    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor(async editor => {
        if (
          editor &&
          isRustEditor(editor) &&
          isDocumentInWorkspace(editor.document)
        ) {
          log(`Opening ${editor.document.fileName}`);
          this.openEditor(editor);
        }
      })
    );

    this.disposables.push(
      vscode.workspace.onDidSaveTextDocument(async document => {
        const editor = vscode.window.activeTextEditor;
        if (
          editor &&
          isRustEditor(editor) &&
          editor.document === document &&
          isDocumentInWorkspace(editor.document)
        ) {
          this.cache.havoc();
          this.view!.havoc();
          if (this.activeRustEditor) {
            this.openEditor(this.activeRustEditor);
          }
        }
      })
    );

    log("Argus backend setup complete");
  }

  dispose() {
    this.cache.havoc();
    this.view?.cleanup();
    this.ictxt.dispose();
    _.forEach(this.commandDisposables, d => d.dispose());
    _.forEach(this.disposables, d => d.dispose());
    this.commandDisposables = [];
    this.disposables = [];
  }

  get activeRustEditor(): RustEditor | undefined {
    const editor = vscode.window.activeTextEditor;
    return editor && isRustEditor(editor) ? editor : undefined;
  }

  get extensionPath(): string {
    return this.ictxt.extCtx.extensionPath;
  }

  get visibleEditors(): RustEditor[] {
    return _.filter(vscode.window.visibleTextEditors, isRustEditor);
  }

  // NOTE: callbacks that register events in this function should invoke
  // actions on the `globals.ctx` instead of `this` to avoid issues with
  // setup. Previously the setup failed but callbacks were still registered
  // with `this` which caused the editor to be out of sync.
  // FIXME: this probably demonstrates a flaw in the design anyways...
  async setupBackend() {
    log("Argus backend preloaded");
  }

  async inspectAt(target?: ErrorJumpTargetInfo) {
    if (!this.view) {
      log("Creating panel...");
      const initialData = await this.getFreshWebViewData();
      this.view = new View(this, initialData, target, () => {
        this.view = undefined;
        this.cache.havoc();
      });
    }
    log("Revealing panel");
    this.view.getPanel.reveal();
  }

  async pinMBData() {
    this.view?.sendPinRequest();
  }

  async unpinMBData() {
    this.view?.sendUnpinRequest();
  }

  async openError(target: ErrorJumpTargetInfo) {
    await this.inspectAt(target);
    // SAFETY: if the view is not initialized after
    // calling `inspectAt` there is definitely a bug somewhere.
    await this.view!.blingObligation(target);
  }

  private async openEditor(editor: RustEditor) {
    // Load the obligations in the file, while we have the editor.
    const [obl, sig] = await this.loadObligations(editor);
    if (obl) {
      if (this.view === undefined) {
        log("View not initialized, skipping editor open.");
      }
      return this.view?.openEditor(editor, sig, obl);
    }
  }

  private findVisibleEditorByName(name: Filename): RustEditor | undefined {
    return _.find(this.visibleEditors, e => e.document.fileName === name);
  }

  // Here we load the obligations for a file, and cache the results,
  // there's a distinction between having an editor and not because
  // we only have definitive access to the editor when it's visible.
  private async loadObligations(editor: RustEditor) {
    const [obligationsP, sig] = this.cache.getObligationsInBody(
      editor.document.fileName
    );

    const obligations = await obligationsP;
    if (obligations !== undefined) this.refreshDiagnostics(editor, obligations);

    return [obligations, sig] as const;
  }

  async getObligations(filename: Filename) {
    return this.cache.getObligationsInBody(filename);
  }

  async getTree(filename: Filename, obligation: Obligation, range: CharRange) {
    return this.cache.getTreeForObligation(filename, obligation, range);
  }

  cancelRunningTasks() {
    this.cache.havoc();
  }

  // ------------------------------------
  // Diagnostic helpers

  private async provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    _token: vscode.CancellationToken
  ) {
    if (!isRustDocument(document) || !isDocumentInWorkspace(document)) {
      log("Document not in workspace", document);
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

    const [infoP, _sig] = this.cache.getObligationsInBody(document.fileName);
    const info = (await infoP) ?? [];

    const traitRecs: Rec[] = _.flatMap(info, ob =>
      _.map(ob.traitErrors, e => ({
        body: ob,
        bidx: ob.hash,
        eidx: e.idx,
        hashes: e.hashes,
        range: e.range,
        type: "trait"
      }))
    );
    const ambiRecs: Rec[] = _.flatMap(info, ob => {
      return _.map(ob.ambiguityErrors, e => ({
        body: ob,
        bidx: ob.hash,
        eidx: e.idx,
        hashes: [ob.obligations[ob.exprs[e.idx].obligations[0]].hash],
        range: e.range,
        type: "ambig"
      }));
    });
    const recs = _.concat(traitRecs, ambiRecs);
    const messages = _.map(recs, rec => {
      const range = rustRangeToVscodeRange(rec.range);

      if (!range.contains(position)) {
        log("Skipping position outside of range", range, position);
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
      contents: _.compact(messages)
    };
  }

  private refreshDiagnostics(editor: RustEditor, info: ObligationsInBody[]) {
    this.diagnosticCollection.clear();

    const diags = _.flatMap(info, ob => {
      // NOTE: the false is to hide superfluous errors.
      const body = new BodyInfo(ob, false);
      const traitDiags = _.map(
        body.traitErrors(),
        e =>
          new vscode.Diagnostic(
            rustRangeToVscodeRange(e.range),
            diagnosticMessage("trait"),
            vscode.DiagnosticSeverity.Error
          )
      );

      const ambigDiags = _.map(
        body.ambiguityErrors(),
        e =>
          new vscode.Diagnostic(
            rustRangeToVscodeRange(e.range),
            diagnosticMessage("ambig"),
            vscode.DiagnosticSeverity.Error
          )
      );
      return [...traitDiags, ...ambigDiags];
    });

    this.diagnosticCollection.set(editor.document.uri, diags);
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
    mdStr.appendText(`${msg}\n`);
    _.forEach(highlightUris, uri =>
      mdStr.appendMarkdown(`- [Open failure in Argus](${uri})\n`)
    );
    return mdStr;
  }

  async _addHighlightRange(editor: RustEditor, range: CharRange) {
    this.highlightRanges.push(range);
    await this.refreshHighlights(editor);
  }

  async addHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findVisibleEditorByName(filename);
    if (editor) await this._addHighlightRange(editor, range);
  }

  async _removeHighlightRange(editor: RustEditor, range: CharRange) {
    this.highlightRanges = _.filter(
      this.highlightRanges,
      r => !_.isEqual(r, range)
    );
    await this.refreshHighlights(editor);
  }

  async removeHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findVisibleEditorByName(filename);
    if (editor) await this._removeHighlightRange(editor, range);
  }

  async jumpToDef(location: DefLocation) {
    let uri: vscode.Uri;
    try {
      uri = vscode.Uri.file(location.f);
    } catch (e: any) {
      // TODO: files from the current crate default to paths from `src/..`
      // but they should be sent fully qualified as well for consistency.
      uri = vscode.Uri.joinPath(this.ictxt.extCtx.extensionUri, location.f);
    }

    const range = rustRangeToVscodeRange(location.r);

    // Open the document at the URI
    const document = await vscode.workspace.openTextDocument(uri);

    // Show the document and move the cursor to the position
    const editor = await vscode.window.showTextDocument(document, {
      // TODO: this is always the first, but what if Argus is in the first.
      // ideally it doesn't cover up argus.
      viewColumn: vscode.ViewColumn.One
    });

    if (isRustEditor(editor)) {
      await this._addHighlightRange(editor, location.r);
      setTimeout(() => this._removeHighlightRange(editor, location.r), 5000);
    }

    editor.revealRange(range);
  }

  private async refreshHighlights(editor: RustEditor) {
    editor.setDecorations(
      rangeHighlight,
      _.map(this.highlightRanges, r => rustRangeToVscodeRange(r))
    );
  }

  private async getFreshWebViewData() {
    const initialData = await Promise.all(
      _.map(this.visibleEditors, async e => {
        const fn = e.document.fileName;
        const [pr, signature] = this.cache.getObligationsInBody(
          e.document.fileName
        );
        const data = await pr;
        if (data) {
          return { fn, data, signature } as FileInfo;
        }
      })
    );
    return _.compact(initialData);
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

// Aliases to keep type coercions shorter.
type PTuple = [CPromise<ObligationsInBody[] | undefined>, string];
type TreeRecord = Record<ObligationHash, CPromise<SerializedTree | undefined>>;
class BackendCache {
  private obligationCache: Record<Filename, PTuple>;
  private treeCache: Record<Filename, TreeRecord>;
  private backend: CallArgus;

  constructor(
    readonly statusBar: StatusBar,
    backend: CallArgus
  ) {
    this.obligationCache = {};
    this.treeCache = {};
    this.backend = backend;
  }

  havoc() {
    _.forEach(this.obligationCache, ([p, _]) => p.cancel());
    _.forEach(this.treeCache, t => _.forEach(t, p => p.cancel()));
    this.obligationCache = {};
    this.treeCache = {};
  }

  getObligationsInBody(filename: Filename) {
    if (this.obligationCache[filename] !== undefined) {
      return this.obligationCache[filename];
    }

    const thunk = this.backend<"obligations">(["obligations", filename]).then(
      res => {
        if (res.type !== "output") {
          this.statusBar.setState("error");
          // NOTE: we probably should, but don't, report this as a bug
          // to the user and open an error dialog box. The reason for this
          // is a malformed `Cargo.toml` can cause errors within `rustc_plugin`.
          // This in turn causes Argus to fail but it isn't really a "failure"
          // on Argus' part.
          log(res.error);
          return;
        }
        return res.value;
      }
    );

    this.obligationCache[filename] = [thunk, makeid(8)] as PTuple;
    return this.obligationCache[filename];
  }

  getTreeForObligation(filename: Filename, obl: Obligation, range: CharRange) {
    if (this.treeCache[filename] !== undefined) {
      if (this.treeCache[filename][obl.hash] !== undefined) {
        return this.treeCache[filename][obl.hash];
      }
    } else {
      this.treeCache[filename] = {};
    }

    const thunk = this.backend<"tree">([
      "tree",
      filename,
      obl.hash,
      range.start.line,
      range.start.column,
      range.end.line,
      range.end.column
    ]).then(res => {
      if (res.type !== "output") {
        this.statusBar.setState("error");
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
        this.statusBar.setState("error");
        return;
      }

      return tree0;
    });

    this.treeCache[filename][obl.hash] = thunk;
    return this.treeCache[filename][obl.hash];
  }
}
