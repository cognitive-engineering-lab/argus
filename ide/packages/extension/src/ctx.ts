import {
  AmbiguityError,
  CharRange,
  ObligationHash,
  TraitError,
} from "@argus/common/bindings";
import { ArgusArgs, ArgusResult, Filename } from "@argus/common/lib";
import _ from "lodash";
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
  private _highlightRanges: CharRange[] = [];
  private _commandDisposables: Disposable[];
  private _commandFactories: Record<string, CommandFactory>;
  private _workspace: Workspace;
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
    this._commandDisposables = [];
    this._commandFactories = commandFactories;
    this._workspace = workspace;
  }

  dispose() {
    // TODO: all disposables should be disposed of here.
    _.forEach(this._commandDisposables, d => d.dispose());
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
    this._backend = b;

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

  setTraitErrors(filename: Filename, errors: TraitError[]) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    const renderBlingCommand = (oblHash: ObligationHash) => {
      const blingCommandUri = vscode.Uri.parse(
        `command:argus.blingObligation?${encodeURIComponent(
          JSON.stringify([filename, oblHash])
        )}`
      );
      return `[Jump to](${blingCommandUri})`;
    };

    const renderErrorAction = (err: TraitError): vscode.MarkdownString => {
      // TODO: make the hover message useful and structured.
      const text = _.join(
        _.map(err.candidates, candHash => {
          return "Candidate obligation: " + renderBlingCommand(candHash) + "\n";
        }),
        "___"
      );
      const result = new vscode.MarkdownString(text);
      result.isTrusted = true;
      return result;
    };

    editor.setDecorations(
      traitErrorDecorate,
      _.map(errors, e => {
        return {
          range: rustRangeToVscodeRange(e.range),
          hoverMessage: renderErrorAction(e),
        };
      })
    );
  }

  setAmbiguityErrors(filename: Filename, errors: AmbiguityError[]) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    const renderErrorAction = (err: AmbiguityError): vscode.MarkdownString => {
      // TODO: make the hover message useful and structured.
      const text = "This is **aMbiGuOuS** ¯\\_(ツ)_/¯";
      const result = new vscode.MarkdownString(text);
      result.isTrusted = true;
      return result;
    };

    editor.setDecorations(
      ambigErrorDecorate,
      _.map(errors, e => {
        return {
          range: rustRangeToVscodeRange(e.range),
          hoverMessage: renderErrorAction(e),
        };
      })
    );
  }

  async addHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    this._highlightRanges.push(range);
    await this._refreshHighlights(editor);
  }

  async removeHighlightRange(filename: Filename, range: CharRange) {
    const editor = this.findOpenEditorByName(filename);
    if (editor === undefined) {
      return;
    }

    this._highlightRanges = _.filter(
      this._highlightRanges,
      r => !_.isEqual(r, range)
    );
    await this._refreshHighlights(editor);
  }

  private async _refreshHighlights(editor: RustEditor) {
    editor.setDecorations(
      rangeHighlight,
      _.map(this._highlightRanges, r => rustRangeToVscodeRange(r))
    );
  }

  private _updateCommands() {
    this._commandDisposables.forEach(disposable => disposable.dispose());
    this._commandDisposables = [];
    for (const [name, factory] of Object.entries(this._commandFactories)) {
      const fullName = `argus.${name}`;
      let callback = factory.enabled(this);
      this._commandDisposables.push(
        vscode.commands.registerCommand(fullName, callback)
      );
    }
  }
}
