import React from "react";

import "./Icons.css";
import "@vscode/codicons/dist/codicon.css";
import classNames from "classnames";

type ButtonProps = {
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void;
};

const codicon =
  (name: string) =>
  (props: React.HTMLAttributes<HTMLElement> & ButtonProps) => (
    <i
      {...props}
      className={classNames("codicon", `codicon-${name}`, props.className)}
    />
  );

// NOTE: not an exhaustive list of call vscode codicons, just add them when necessary.

export const IcoTriangleRight = codicon("triangle-right");
export const IcoTriangleDown = codicon("triangle-down");

export const IcoChevronDown = codicon("chevron-down");
export const IcoChevronUp = codicon("chevron-up");
export const IcoChevronLeft = codicon("chevron-left");
export const IcoChevronRight = codicon("chevron-right");

export const IcoCheck = codicon("check");
export const IcoError = codicon("error");
export const IcoAmbiguous = codicon("question");

export const IcoNote = codicon("note");
export const IcoComment = codicon("comment");

export const IcoPlus = codicon("plus");
export const IcoDot = codicon("circle-small-filled");
export const IcoLoop = codicon("sync");
export const IcoMegaphone = codicon("megaphone");
export const IcoEyeClosed = codicon("eye-closed");
export const IcoLock = codicon("lock");
export const IcoTreeDown = codicon("type-hierarchy-sub");
export const IcoPinned = codicon("pinned");
export const IcoListUL = codicon("list-unordered");
export const IcoSettingsGear = codicon("settings-gear");
