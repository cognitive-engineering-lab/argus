import React from "react";

import "./Icons.css";

type ButtonProps = {
  onClick?: (event: React.MouseEvent<HTMLButtonElement>) => void;
};

const makeCodicon =
  (name: string) =>
  (props: React.HTMLAttributes<HTMLElement> & ButtonProps) => (
    <i className={`codicon codicon-${name}`} {...props} />
  );

// NOTE: not an exhaustive list of call vscode codicons, just add them when necessary.

export const IcoTriangleRight = makeCodicon("triangle-right");
export const IcoTriangleDown = makeCodicon("triangle-down");

export const IcoChevronDown = makeCodicon("chevron-down");
export const IcoChevronUp = makeCodicon("chevron-up");
export const IcoChevronLeft = makeCodicon("chevron-left");
export const IcoChevronRight = makeCodicon("chevron-right");

export const IcoCheck = makeCodicon("check");
export const IcoError = makeCodicon("error");
export const IcoAmbiguous = makeCodicon("question");

export const IcoNote = makeCodicon("note");
export const IcoComment = makeCodicon("comment");

export const IcoPlus = makeCodicon("plus");
export const IcoDot = makeCodicon("circle-small-filled");
export const IcoLoop = makeCodicon("sync");
export const IcoMegaphone = makeCodicon("megaphone");
export const IcoEyeClosed = makeCodicon("eye-closed");
export const IcoLock = makeCodicon("lock");
export const IcoTreeDown = makeCodicon("type-hierarchy-sub");
export const IcoPinned = makeCodicon("pinned");
