import React, { type PropsWithChildren } from "react";

import "./MonoSpace.css";

const MonoSpace = ({ children, idx }: PropsWithChildren<{ idx?: string }>) => (
  <span className="MonoSpaceArea" data-idx={idx}>
    {children}
  </span>
);

export default MonoSpace;
