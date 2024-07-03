import classNames from "classnames";
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
    <div
      className={classNames("toggle-box", { expanded })}
      onClick={e => {
        e.stopPropagation();
        setExpanded(!expanded);
      }}
    >
      {expanded ? <Children /> : <span className="summary">{summary}</span>}
    </div>
  );
};
