import classNames from "classnames";
import React, { useState } from "react";

import "./Toggle.css";

export let Toggle = ({
  Children,
  summary,
}: {
  summary: React.ReactNode;
  Children: React.FC;
}) => {
  let [expanded, setExpanded] = useState(false);
  return (
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
