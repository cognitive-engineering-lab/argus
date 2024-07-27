import type {
  DefinedPath,
  ExtensionCandidates,
  GoalData,
  ImplHeader,
  Obligation,
  Ty,
  TyVal
} from "@argus/common/bindings";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";

import ErrorDiv from "./ErrorDiv";
import MonoSpace from "./MonoSpace";
import ReportBugUrl from "./ReportBugUrl";
import "./lib.css";
import { AllowToggle } from "./context";
import { PrintImplHeader as UnsafePrintImplHeader } from "./private/argus";
import { PrintDefinitionPath as UnsafePrintDefPath } from "./private/path";
import {
  PrintGoalPredicate as UnsafePrintGoalPredicate,
  PrintPredicateObligation as UnsafePrintPredicateObligation
} from "./private/predicate";
import {
  PrintTy as UnsafePrintTy,
  PrintTyValue as UnsafePrintTyValue
} from "./private/ty";

// NOTE: please Please PLEASE wrap all printing components in this
// `PrintWithFallback`. Pretty printing is still a fragile process and
// I don't have full confidence in it yet.
//
// Additionally, this component sets the contents to stlye with the editor monospace font.
export const PrintWithFallback = ({
  object,
  Content
}: {
  object: any;
  Content: React.FC;
}) => {
  const FallbackFromError = ({
    error,
    resetErrorBoundary: _
  }: {
    error: any;
    resetErrorBoundary: (...args: any[]) => void;
  }) => {
    if (object === undefined) {
      return "(ERR: undef)";
    }

    // NOTE: Call resetErrorBoundary() to reset the error boundary and retry the render.
    return (
      <ErrorDiv>
        Whoops! Something went wrong while printing. This is a bug, please{" "}
        <ReportBugUrl
          error={error.message}
          displayText="click here"
          logText={JSON.stringify(object)}
        />{" "}
        to report it.
      </ErrorDiv>
    );
  };

  return (
    <ErrorBoundary
      FallbackComponent={FallbackFromError}
      onReset={details => {
        console.error(details);
      }}
    >
      <MonoSpace>
        <Content />
      </MonoSpace>
    </ErrorBoundary>
  );
};

export const PrintDefPath = ({ defPath }: { defPath: DefinedPath }) => (
  <PrintWithFallback
    object={defPath}
    Content={() => <UnsafePrintDefPath o={defPath} />}
  />
);

export const PrintTy = ({ ty }: { ty: Ty }) => (
  <PrintWithFallback object={ty} Content={() => <UnsafePrintTy o={ty} />} />
);

export const PrintObligation = ({ obligation }: { obligation: Obligation }) => {
  const InnerContent = () => (
    <AllowToggle.Provider value={true}>
      <UnsafePrintPredicateObligation o={obligation.obligation} />
    </AllowToggle.Provider>
  );
  return <PrintWithFallback object={obligation} Content={InnerContent} />;
};

export const PrintImplHeader = ({ impl }: { impl: ImplHeader }) => (
  <AllowToggle.Provider value={true}>
    <PrintWithFallback
      object={impl}
      Content={() => <UnsafePrintImplHeader o={impl} />}
    />
  </AllowToggle.Provider>
);

export const PrintGoal = ({ o }: { o: GoalData }) => {
  const debugString =
    o.debugComparison === undefined ? null : (
      <div style={{ opacity: 0.5 }}>{o.debugComparison}</div>
    );
  const Content = () => (
    <AllowToggle.Provider value={true}>
      <UnsafePrintGoalPredicate o={o.value} />
      {debugString}
    </AllowToggle.Provider>
  );
  return <PrintWithFallback object={o} Content={Content} />;
};

// The individual components aren't typed, so we'll require passing the entire array for now.
export const PrintExtensionCandidate = ({
  candidates,
  idx
}: {
  candidates: ExtensionCandidates;
  idx: number;
}) => {
  const o = candidates.data[idx];
  return o === undefined ? (
    "?"
  ) : (
    <PrintWithFallback
      object={o}
      Content={() => <UnsafePrintDefPath o={o} />}
    />
  );
};

export const PrintBodyName = ({ defPath }: { defPath: DefinedPath }) => (
  <PrintWithFallback
    object={defPath}
    Content={() => <UnsafePrintDefPath o={defPath} />}
  />
);

export const PrintTyValue = ({ ty }: { ty: TyVal }) => (
  <PrintWithFallback
    object={ty}
    Content={() => <UnsafePrintTyValue o={ty} />}
  />
);
