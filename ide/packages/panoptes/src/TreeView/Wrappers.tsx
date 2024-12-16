import type { CandidateIdx, ProofNodeIdx } from "@argus/common/bindings";
import type {
  InfoWrapper,
  InfoWrapperProps
} from "@argus/common/communication";
import { TreeAppContext } from "@argus/common/context";
import { arrUpdate } from "@argus/common/func";
import { IcoListUL, IcoTreeDown } from "@argus/print/Icons";
import {} from "@floating-ui/react";
import classNames from "classnames";
import _ from "lodash";
import React, { type ReactElement, useState, useContext } from "react";
import Graph from "./Graph";
import { Candidate } from "./Node";

import "./Wrappers.css";
import { PrintDefPath } from "@argus/print/lib";
import Floating from "../Floating";

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

const DetailsPortal = ({
  children,
  info,
  reportActive
}: React.PropsWithChildren<
  Omit<InfoWrapperProps, "n"> & { info: ReactElement }
>) => (
  <Floating
    outerClassName="tree-toggle"
    innerClassName="floating-graph"
    toggle={info}
    reportActive={reportActive}
  >
    {children}
  </Floating>
);

export const WrapTreeIco = ({ n, reportActive }: InfoWrapperProps) => (
  <DetailsPortal reportActive={reportActive} info={<IcoTreeDown />}>
    <Graph root={n} />
  </DetailsPortal>
);

export const WrapImplCandidates = ({ n, reportActive }: InfoWrapperProps) => {
  const tree = useContext(TreeAppContext.TreeContext)!;
  const implementors = tree.implCandidates(n);
  if (implementors === undefined) return null;
  const totalImpls =
    implementors.impls.length + implementors.inductiveImpls.length;

  if (totalImpls === 0) return null;

  const Section = ({ candidates }: { candidates: CandidateIdx[] }) =>
    _.map(candidates, (c, i) => (
      <div key={i}>
        <Candidate idx={c} />
      </div>
    ));

  return (
    <DetailsPortal reportActive={reportActive} info={<IcoListUL />}>
      <p>
        There are {totalImpls} <PrintDefPath defPath={implementors.trait} />{" "}
        implementors
      </p>
      <div className="ImplCandidatesPanel">
        <Section candidates={implementors.impls} />
        <Section candidates={implementors.inductiveImpls} />
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
