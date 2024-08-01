import type { ProofNodeIdx } from "@argus/common/bindings";
import type {
  InfoWrapper,
  InfoWrapperProps
} from "@argus/common/communication";
import { TreeAppContext } from "@argus/common/context";
import { IcoListUL, IcoTreeDown } from "@argus/print/Icons";
import { PrintImplHeader } from "@argus/print/lib";
import {
  FloatingFocusManager,
  FloatingPortal,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions
} from "@floating-ui/react";
import classNames from "classnames";
import _ from "lodash";
import React, { useContext, useState } from "react";
import Graph from "./Graph";

import "./Wrappers.css";

export const WrapNode = ({
  children,
  wrappers,
  n
}: React.PropsWithChildren<{ wrappers: InfoWrapper[]; n: ProofNodeIdx }>) => {
  const [hovered, setHovered] = useState(false);
  const [actives, setActives] = _.unzip(
    _.map(wrappers, _w => useState(false))
  ) as [boolean[], React.Dispatch<React.SetStateAction<boolean>>[]];

  const active = _.some(actives);

  return (
    <span
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {children}
      {(hovered || active) && (
        <span className="WrapperBox">
          {_.map(wrappers, (W, i) => (
            <W key={i} n={n} reportActive={setActives[i]} />
          ))}
        </span>
      )}
    </span>
  );
};

const composeEvents =
  <T,>(...es: ((t: T) => void)[]) =>
  (t: T) =>
    _.forEach(es, e => e(t));

export const WrapTreeIco = ({ n, reportActive }: InfoWrapperProps) => {
  const [isOpen, setIsOpen] = useState(false);
  const openCallback = composeEvents(setIsOpen, reportActive);

  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: openCallback,
    placement: "bottom",
    middleware: [offset(() => 5), shift()]
  });

  const click = useClick(context);
  const dismiss = useDismiss(context);
  const { getReferenceProps, getFloatingProps } = useInteractions([
    click,
    dismiss
  ]);

  return (
    <>
      <span
        className="tree-toggle"
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        <IcoTreeDown />
      </span>
      {isOpen && (
        <FloatingPortal>
          <FloatingFocusManager context={context}>
            <div
              className={classNames("floating", "floating-graph")}
              ref={refs.setFloating}
              style={floatingStyles}
              {...getFloatingProps()}
            >
              <Graph root={n} />
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
};

export const WrapImplCandidates = ({ n, reportActive }: InfoWrapperProps) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const candidates = tree.implCandidates(n);

  if (candidates === undefined || candidates.length === 0) {
    return null;
  }

  const [isOpen, setIsOpen] = useState(false);
  const openCallback = composeEvents(setIsOpen, reportActive);

  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: openCallback,
    placement: "bottom",
    middleware: [offset(() => 5), shift()]
  });

  const click = useClick(context);
  const dismiss = useDismiss(context);
  const { getReferenceProps, getFloatingProps } = useInteractions([
    click,
    dismiss
  ]);

  return (
    <>
      <span
        className="tree-toggle"
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        <IcoListUL />
      </span>
      {isOpen && (
        <FloatingPortal>
          <FloatingFocusManager context={context}>
            <div
              className={classNames("floating", "floating-graph")}
              ref={refs.setFloating}
              style={floatingStyles}
              {...getFloatingProps()}
            >
              <p>
                The following {candidates.length} implementations are available:
              </p>
              <div className="ImplCandidatesPanel">
                {_.map(candidates, (c, i) => (
                  <div key={i}>
                    <PrintImplHeader impl={c} />
                  </div>
                ))}
              </div>
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
};

export const mkJumpToTopDownWrapper =
  (jumpTo: (n: ProofNodeIdx) => void) =>
  ({ n }: InfoWrapperProps) => {
    const jumpToTree = (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      jumpTo(n);
    };

    return <IcoTreeDown className="tree-toggle" onClick={jumpToTree} />;
  };
