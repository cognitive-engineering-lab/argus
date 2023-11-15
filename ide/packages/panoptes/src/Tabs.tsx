import React, { useState } from 'react';
import "./Tabs.css";

let TabGroup = ({
  components,
}: {
  components: [string, React.ReactNode][];
}) => {
  const [[activen, activec], setActive] = useState(components[0]);
  return (
    <div className="TabContainer">
      <div className="TabGroup">
        {components.map(([name, component], idx) => (
          <button
            key={idx}
            className={activen === name ? "active" : ""}
            onClick={() => setActive([name, component])}
          >
            {name}
          </button>
        ))}
      </div>
      <div className="TabContentArea">{activec}</div>
    </div>
  );
};

export default TabGroup;