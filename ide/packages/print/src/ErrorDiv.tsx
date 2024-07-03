import React from "react";

import "./ErrorDiv.css";

const ErrorDiv = ({ children }: React.PropsWithChildren<{}>) => (
  <div className="ErrorDiv">{children}</div>
);

export default ErrorDiv;
