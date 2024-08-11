import type { ProofNodeIdx } from "@argus/common/bindings";
import type {
  InfoWrapper,
  InfoWrapperProps
} from "@argus/common/communication";
import { TreeAppContext } from "@argus/common/context";
import { arrUpdate } from "@argus/common/func";
import { IcoListUL, IcoTreeDown } from "@argus/print/Icons";
import { PrintImplHeader } from "@argus/print/lib";
import {
  FloatingArrow,
  FloatingFocusManager,
  FloatingPortal,
  arrow,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions
} from "@floating-ui/react";
import classNames from "classnames";
import _ from "lodash";
import React, { type ReactElement, useState, useContext, useRef } from "react";
import Graph from "./Graph";

import "./Wrappers.css";

export const WrapNode = ({
  children,
  wrappers,
  n
}: React.PropsWithChildren<{ wrappers: InfoWrapper[]; n: ProofNodeIdx }>) => {
  const [hovered, setHovered] = useState(false);
  const [actives, setActives] = useState(Array(wrappers.length).fill(false));

  const active = _.some(actives);
  const className = classNames("WrapperBox", {
    "is-hovered": hovered || active
  });

  return (
    <span
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {children}
      <span className={className}>
        {_.map(wrappers, (W, i) => (
          <W
            key={i}
            n={n}
            reportActive={b => setActives(a => arrUpdate(a, i, b))}
          />
        ))}
      </span>
    </span>
  );
};

const composeEvents =
  <T,>(...es: ((t: T) => void)[]) =>
  (t: T) =>
    _.forEach(es, e => e(t));

const DetailsPortal = ({
  children,
  info,
  reportActive
}: React.PropsWithChildren<
  Omit<InfoWrapperProps, "n"> & { info: ReactElement }
>) => {
  const [isOpen, setIsOpen] = useState(false);
  const openCallback = composeEvents(setIsOpen, reportActive);
  const arrowRef = useRef(null);

  const ARROW_HEIGHT = 10;
  const GAP = 5;

  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: openCallback,
    placement: "bottom",
    middleware: [
      offset(ARROW_HEIGHT + GAP),
      shift(),
      arrow({
        element: arrowRef
      })
    ]
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
        {info}
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
              <FloatingArrow
                ref={arrowRef}
                context={context}
                height={ARROW_HEIGHT}
                tipRadius={3}
                stroke="2"
              />
              {children}
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
};

export const WrapTreeIco = ({ n, reportActive }: InfoWrapperProps) => (
  <DetailsPortal reportActive={reportActive} info={<IcoTreeDown />}>
    <Graph root={n} />
  </DetailsPortal>
);

export const WrapImplCandidates = ({ n, reportActive }: InfoWrapperProps) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const candidates = tree.implCandidates(n);

  if (candidates === undefined || candidates.length === 0) {
    return null;
  }

  return (
    <DetailsPortal reportActive={reportActive} info={<IcoListUL />}>
      <p>The following {candidates.length} implementations are available:</p>
      <div className="ImplCandidatesPanel">
        {_.map(candidates, (c, i) => (
          <div key={i}>
            <PrintImplHeader impl={c} />
          </div>
        ))}
      </div>
    </DetailsPortal>
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
