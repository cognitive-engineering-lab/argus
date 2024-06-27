import React, { type PropsWithChildren } from "react";

const MonoSpace = ({ children }: PropsWithChildren) => (
  <span
    style={{
      fontFamily: "var(--vscode-editor-font-family)",
      fontSize: "var(--vscode-editor-font-size)"
    }}
  >
    {children}
  </span>
);

export default MonoSpace;
