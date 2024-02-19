import vscode from "vscode";

export type StatusBarState =
  | "active"
  | "unsaved"
  | "idle"
  | "error"
  | "loading"
  | "notfound";

interface StatusBarConfig {
  foreground: string;
  background: string;
  icon?: string;
  command: string;
  tooltip?: string;
}

const defaultConfigs: Record<StatusBarState, StatusBarConfig> = {
  active: {
    foreground: "statusBarItem.warningForeground",
    background: "statusBarItem.warningBackground",
    icon: "check",
    command: "argus.inspectWorkspace",
  },
  unsaved: {
    foreground: "statusBarItem.foreground",
    background: "statusBarItem.background",
    icon: "circle-slash",
    command: "argus.inspectWorkspace",
  },
  idle: {
    foreground: "statusBarItem.foreground",
    background: "statusBarItem.background",
    command: "argus.inspectWorkspace",
  },
  error: {
    foreground: "statusBarItem.errorForeground",
    background: "statusBarItem.errorBackground",
    icon: "x",
    command: "argus.lastError",
  },
  loading: {
    foreground: "statusBarItem.foreground",
    background: "statusBarItem.background",
    icon: "sync~spin",
    command: "argus.inspectWorkspace",
  },
  notfound: {
    foreground: "statusBarItem.foreground",
    background: "statusBarItem.background",
    icon: "question",
    command: "argus.inspectWorkspace",
    tooltip:
      "Argus could not get Cargo to find this file (this is probably a Argus bug)",
  },
};

export class StatusBar {
  bar: vscode.StatusBarItem;
  state: StatusBarState = "loading";

  constructor(
    context: vscode.ExtensionContext,
    readonly name: string = "argus",
    readonly configs: Record<StatusBarState, StatusBarConfig> = defaultConfigs
  ) {
    this.bar = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left
    );
    context.subscriptions.push(this.bar);
    this.bar.show();
  }

  setState(state: StatusBarState, tooltip: string = "") {
    this.state = state;
    this.bar.tooltip = tooltip;
    this.render();
  }

  render() {
    const config = this.configs[this.state];
    this.bar.color = config.foreground;
    this.bar.backgroundColor = new vscode.ThemeColor(config.background);
    this.bar.text = `$(${config.icon}) ${this.name}`;
    this.bar.command = config.command;
    this.bar.tooltip = config.tooltip;
  }
}
