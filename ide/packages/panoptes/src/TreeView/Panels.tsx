import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useState, type ReactElement } from "react";

import "./Panels.css";

export type TabProps = React.ComponentPropsWithRef<typeof VSCodePanelTab>;
export type ViewProps = React.ComponentPropsWithRef<typeof VSCodePanelView>;
export type PanelsProps = React.ComponentPropsWithRef<typeof VSCodePanels>;

export interface PanelDescription {
  title: string;
  Content: ReactElement;
  tabProps?: TabProps;
  viewProps?: ViewProps;
}

const Panels = ({
  manager,
  description
}: {
  manager?: [number, (n: number) => void];
  description: PanelDescription[];
}) => {
  const [active, setActive] = manager || useState(0);

  const tabId = (n: number) => `tab-${n}`;
  console.warn("Starting panels on", active);

  const withExtra = _.map(description, (d, idx) => ({
    ...d,
    id: tabId(idx),
    onClick: () => setActive(idx)
  }));

  return (
    <VSCodePanels activeid={tabId(active)}>
      {_.map(withExtra, ({ id, title, onClick, tabProps }, idx) => (
        <VSCodePanelTab {...tabProps} key={idx} id={id} onClick={onClick}>
          {title}
        </VSCodePanelTab>
      ))}
      {_.map(withExtra, ({ Content, viewProps }, idx) => (
        <VSCodePanelView {...viewProps} key={idx}>
          {Content}
        </VSCodePanelView>
      ))}
    </VSCodePanels>
  );
};

export default Panels;
