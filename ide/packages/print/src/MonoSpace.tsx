import React, { type PropsWithChildren } from "react";

import "./MonoSpace.css";

const MonoSpace = ({ children }: PropsWithChildren) => (
  <span className="MonoSpaceArea">
    {children}
  </span>
);

export default MonoSpace;
