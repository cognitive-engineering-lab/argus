import React, { useState } from "react";

import "./Toggle.css";

export const Toggle = ({
  Children,
  summary
}: {
  summary: React.ReactNode;
  Children: React.FC;
}) => {
  const [expanded, setExpanded] = useState(false);
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: TODO
    <details
      className="toggle-box"
      open={expanded}
      onClick={e => {
        e.preventDefault();
        e.stopPropagation();
        setExpanded(e => !e);
      }}
    >
      <summary>{summary}</summary>
      <Children />
    </details>
  );
};
