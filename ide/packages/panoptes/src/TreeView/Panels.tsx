import { TextEmphasis } from "@argus/print/Attention";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels,
  VSCodeProgressRing
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { Suspense, useState } from "react";

import "./Panels.css";

export type TabProps = React.ComponentPropsWithRef<typeof VSCodePanelTab>;
export type ViewProps = React.ComponentPropsWithRef<typeof VSCodePanelView>;
export type PanelsProps = React.ComponentPropsWithRef<typeof VSCodePanels>;

export interface PanelDescription {
  title: string;
  Content: React.FC;
  tabProps?: TabProps;
  viewProps?: ViewProps;
}

const Panels = ({
  manager,
  description
}: {
  manager?: [number, (n: number) => void, boolean?];
  description: PanelDescription[];
}) => {
  const [active, setActive, programaticSwitch] = manager || useState(0);

  const tabId = (n: number) => `tab-${n}`;
  console.warn("Starting panels on", active);

  const withExtra = _.map(description, (d, idx) => ({
    ...d,
    id: tabId(idx),
    onClick: () => setActive(idx)
  }));

  return (
    <VSCodePanels activeid={tabId(active)}>
      {_.map(withExtra, ({ id, title, onClick, tabProps }, idx) => {
        const Wrapper =
          programaticSwitch && active === idx ? TextEmphasis : React.Fragment;
        return (
          <VSCodePanelTab {...tabProps} key={idx} id={id} onClick={onClick}>
            <Wrapper>{title}</Wrapper>
          </VSCodePanelTab>
        );
      })}
      {_.map(withExtra, ({ Content, viewProps }, idx) => (
        <VSCodePanelView {...viewProps} key={idx}>
          <Suspense fallback={<VSCodeProgressRing />}>
            <Content />
          </Suspense>
        </VSCodePanelView>
      ))}
    </VSCodePanels>
  );
};

export default Panels;
