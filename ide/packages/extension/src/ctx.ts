import {
  AmbiguityError,
  CharRange,
  ObligationHash,
  ObligationsInBody,
  TraitError,
} from "@argus/common/bindings";
import { ArgusArgs, ArgusResult, Filename } from "@argus/common/lib";
import _ from "lodash";
import { render } from "react-dom";
import * as vscode from "vscode";

import {
  ambigErrorDecorate,
  rangeHighlight,
  traitErrorDecorate,
} from "./decorate";
import { showErrorDialog } from "./errors";
import { ArgusCtx } from "./main";
import { setup } from "./setup";
import {
  RustEditor,
  isRustDocument,
  isRustEditor,
  rustRangeToVscodeRange,
} from "./utils";

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

export class Ctx implements ArgusCtx {
  // Ranges used for highlighting code in the current Rust Editor.
  private highlightRanges: CharRange[] = [];
  private commandDisposables: Disposable[];
  private commandFactories: Record<string, CommandFactory>;
  private workspace: Workspace;
  private errorCollection;

  private _backend: <T>(
    _args: ArgusArgs,
    _no_output?: boolean
  ) => Promise<ArgusResult<T>> = () => {
    throw new Error(
      "The Argus backend in not available. " +
        // TODO: make sure this links to some sort of installation documentation.
        "Please ensure that it has been [properly installed](https://github.com/gavinleroy/argus)."
    );
  };

  constructor(
    readonly extCtx: vscode.ExtensionContext,
    commandFactories: Record<string, CommandFactory>,
    workspace: Workspace
  ) {
    this.commandDisposables = [];
    this.commandFactories = commandFactories;
    this.workspace = workspace;
    this.errorCollection = vscode.languages.createDiagnosticCollection("argus");
  }

  dispose() {
    // TODO: all disposables should be disposed of here.
    _.forEach(this.commandDisposables, d => d.dispose());
  }

  backend<T>(args: ArgusArgs, no_output?: boolean): Promise<ArgusResult<T>> {
    return this._backend(args, no_output);
  }

  get activeRustEditor(): RustEditor | undefined {
    let editor = vscode.window.activeTextEditor;
    return editor && isRustEditor(editor) ? editor : undefined;
  }

  get extensionPath(): string {
    return this.extCtx.extensionPath;
  }

  async setupBackend() {
    let b = await setup(this);
    if (b == null) {
      showErrorDialog("Failed to setup Argus");
      return;
    }
    // TODO: add some sort of "status loading" indicator.
    // Compile the workspace with the Argus version of rustc.
    await b(["preload"], true);
    this.backend = b;

    // Register the commands with VSCode after the backend is setup.
    this._updateCommands();
  }

  get visibleEditors(): RustEditor[] {
    let editors = [];
    for (let editor of vscode.window.visibleTextEditors) {
      if (isRustEditor(editor)) {
        editors.push(editor);
      }
    }
    return editors;
  }

  public findOpenEditorByName(name: Filename): RustEditor | undefined {
    return _.find(this.visibleEditors, e => e.document.fileName === name);
  }

  // TODO: we will want to save the obligations info per file,
  // so changing editors doesn't require a 'reload' of the info.
  registerBodyInfo(filename: Filename, info: ObligationsInBody[]) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    this.setTraitErrors(editor, info);
    this.setAmbiguityErrors(editor, info);
  }

  // ------------------------------------
  // Diagnostic helpers

  private buildOpenErrorItemCmd(
    filename: Filename,
    bodyidx: number,
    erroridx: number,
    type: "trait" | "ambig"
  ): string {
    const blingCommandUri = vscode.Uri.parse(
      `command:argus.openError?${encodeURIComponent(
        JSON.stringify([filename, type, bodyidx, erroridx])
      )}`
    );
    return `[Debug error in window](${blingCommandUri})`;
  }

  private setTraitErrors(editor: RustEditor, oib: ObligationsInBody[]) {
    const renderErrorAction = (
      err: TraitError,
      bIdx: number,
      eIdx: number
    ): vscode.MarkdownString => {
      // TODO: make the hover message useful and structured.
      const jumpToDebug = this.buildOpenErrorItemCmd(
        editor.document.fileName,
        bIdx,
        eIdx,
        "trait"
      );
      const lines: string[] = [
        `Trait bound not satisfied : ${jumpToDebug}`,
      ];
      const result = new vscode.MarkdownString(lines.join("\n"));
      result.isTrusted = true;
      return result;
    };

    editor.setDecorations(
      traitErrorDecorate,
      _.flatMap(oib, (ob, bodyIdx) => {
        return _.map(ob.traitErrors, (e, errIdx) => {
          return {
            range: rustRangeToVscodeRange(e.range),
            hoverMessage: renderErrorAction(e, bodyIdx, errIdx),
          };
        });
      })
    );
  }

  private setAmbiguityErrors(editor: RustEditor, oib: ObligationsInBody[]) {
    const renderErrorAction = (
      err: AmbiguityError,
      bIdx: number,
      eIdx: number
    ): vscode.MarkdownString => {
      // TODO: make the hover message useful and structured.
      const jumpToDebug = this.buildOpenErrorItemCmd(
        editor.document.fileName,
        bIdx,
        eIdx,
        "ambig"
      );
      const lines = ["This method call is ambiguous", "", jumpToDebug];
      const result = new vscode.MarkdownString(lines.join("\n"));
      result.isTrusted = true;
      return result;
    };

    editor.setDecorations(
      ambigErrorDecorate,
      _.flatMap(oib, (oi, bIdx) => {
        return _.map(oi.ambiguityErrors, (e, eIdx) => {
          return {
            range: rustRangeToVscodeRange(e.range),
            hoverMessage: renderErrorAction(e, bIdx, eIdx),
          };
        });
      })
    );
  }

  async addHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    this.highlightRanges.push(range);
    await this._refreshHighlights(editor);
  }

  async removeHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    this.highlightRanges = _.filter(
      this.highlightRanges,
      r => !_.isEqual(r, range)
    );
    await this._refreshHighlights(editor);
  }

  private async _refreshHighlights(editor: RustEditor) {
    editor.setDecorations(
      rangeHighlight,
      _.map(this.highlightRanges, r => rustRangeToVscodeRange(r))
    );
  }

  private _updateCommands() {
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
