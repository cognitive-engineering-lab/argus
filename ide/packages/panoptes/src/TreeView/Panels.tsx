import { arrUpdate } from "@argus/common/func";
import { TextEmphasis } from "@argus/print/Attention";
import {
  VSCodePanelTab,
  VSCodePanelView,
  VSCodePanels
} from "@vscode/webview-ui-toolkit/react";
import _ from "lodash";
import React, { useId, useState, useEffect } from "react";

import "./Panels.css";

export type TabProps = React.ComponentPropsWithRef<typeof VSCodePanelTab>;
export type ViewProps = React.ComponentPropsWithRef<typeof VSCodePanelView>;
export type PanelsProps = React.ComponentPropsWithRef<typeof VSCodePanels>;

export interface PanelDescription {
  title: string;
  Content: React.FC;
  tabProps?: TabProps;
  viewProps?: ViewProps;
  // FIXME: this shouldn't be here, we should require the title to be unique
  fn?: string;
}

interface PanelState {
  activePanel: number;
  node?: number;
  programatic?: boolean;
}

export function usePanelState() {
  const [state, setState] = useState<PanelState>({ activePanel: 0 });
  return [state, setState] as const;
}

// NOTE: we don't expect someone to have more than 15 tabs open...the `VSCodePanels`
// is tricky to navigate so this would be an issue. But, we should make this robust to track
// the size of the underlying buffer better. For safety and performance.
function useStateArray<T>(n = 15) {
  const [values, setValues] = useState<(T | undefined)[]>(
    Array.from({ length: n })
  );
  const updateValue = (idx: number, value: T | undefined) =>
    setValues(a => arrUpdate(a, idx, value));
  const updateAll = (a: (T | undefined)[]) =>
    setValues(Array.from({ ...a, length: n }));
  return [values, updateValue, updateAll] as const;
}

const Panels = ({
  manager,
  description
}: {
  manager?: [number, (n: number) => void, boolean?];
  description: PanelDescription[];
}) => {
  const id = useId();
  const [active, setActive, programaticSwitch] = manager || useState(0);
  const tabId = (n: number) => `tab-${id}-${n}`;

  const [openFiles, setOpenFiles, resetOpenFiles] = useStateArray<string>();
  const [tabs, setTabs, resetTabs] = useStateArray<React.ReactElement>();
  const [panels, setPanels, resetPanels] = useStateArray<React.ReactElement>();

  useEffect(() => {
    console.debug(`Panel(${id}) mounted`);
    resetOpenFiles(_.map(description, d => d.fn ?? d.title));
    fullRender();
  }, []);

  // NOTE: rerenders should not occur if the user clicks on a tab. We cache the
  // elements in state to avoid this. IFF the change is *programatic*, meaning
  // some GUI action caused the change, we always want to force a rerender so that
  // state change visuals are shown.
  useEffect(() => {
    console.debug(`Panel(${id}) params changed`, active, programaticSwitch);
    if (programaticSwitch) {
      // On a programatic switch only rerender the active tab
      rerender(active);
    }
  }, [active, programaticSwitch]);

  // A change in description should always rerender. `useEffect` compares with `Object.is` which
  // returns false for the same valued arrays, a simple hash is the concatenation of all panel titles
  // which is stable across rerenders.
  const descriptionHash = _.reduceRight(
    description,
    (acc, d) => acc + (d.fn ?? d.title),
    ""
  );
  useEffect(() => {
    console.debug(`Panel(${id}) description changed`);
    _.forEach(_.zip(openFiles, description), ([file, d], idx) => {
      if (file === (d?.fn ?? d?.title)) return;

      console.debug("Rerendering due to description change", file, d);
      setOpenFiles(idx, d?.fn ?? d?.title);
      rerender(idx, d);
    });
  }, [descriptionHash]);

  const TWrapper = ({ idx, str }: { idx: number; str: string }) =>
    idx === active && programaticSwitch ? (
      <TextEmphasis>{str}</TextEmphasis>
    ) : (
      str
    );

  const elementsAt = (idx: number, d: PanelDescription) =>
    [
      <VSCodePanelTab
        {...d.tabProps}
        key={idx}
        id={tabId(idx)}
        onClick={() => setActive(idx)}
      >
        <TWrapper idx={idx} str={d.title} />
      </VSCodePanelTab>,
      <VSCodePanelView {...d.viewProps} key={idx}>
        <d.Content />
      </VSCodePanelView>
    ] as const;

  const rerender = (idx: number, desc?: PanelDescription) => {
    if (idx < 0 || description.length <= idx) {
      setTabs(idx, undefined);
      setPanels(idx, undefined);
    }

    const d = desc ?? description[idx];
    const [t, p] = elementsAt(idx, d);
    setTabs(idx, t);
    setPanels(idx, p);
  };

  const fullRender = () => {
    const [ts, ps] = _.unzip(
      _.map(description, (d, idx) => elementsAt(idx, d))
    );
    resetTabs(ts);
    resetPanels(ps);
  };

  return (
    <VSCodePanels activeid={tabId(active)}>
      {tabs}
      {panels}
    </VSCodePanels>
  );
};

export default Panels;
