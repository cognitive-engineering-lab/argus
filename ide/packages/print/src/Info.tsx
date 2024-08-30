import React from "react";

import "./Info.css";

export const ErrorDiv = ({ children }: React.PropsWithChildren) => (
  <div className="ErrorDiv">{children}</div>
);

export const InfoDiv = ({ children }: React.PropsWithChildren) => (
  <div className="InfoDiv">{children}</div>
);
