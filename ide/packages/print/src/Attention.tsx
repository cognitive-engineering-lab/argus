import React from "react";

import "./Attention.css";

export const TextEmphasis = ({ children }: React.PropsWithChildren) => (
  <span className="AttentionText">{children}</span>
);

const Attention = ({ children }: React.PropsWithChildren) => (
  <span className="Attention">{children}</span>
);

export default Attention;
