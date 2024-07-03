import React from "react";

const Indented = ({ children }: { children: React.ReactNode }) => (
  <div style={{ marginLeft: "0.5em" }}>{children}</div>
);

export default Indented;
