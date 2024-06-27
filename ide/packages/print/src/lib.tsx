import type {
  DefinedPath,
  ExtensionCandidates,
  GoalData,
  ImplHeader,
  Obligation,
  Ty
} from "@argus/common/bindings";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";

import ErrorDiv from "./ErrorDiv";
import MonoSpace from "./MonoSpace";
import ReportBugUrl from "./ReportBugUrl";
import "./lib.css";
import { PrintImplHeader as UnsafePrintImplHeader } from "./private/argus";
import { PrintDefPath as UnsafePrintDefPath } from "./private/path";
import { ToggleGenericDelimiterContext } from "./private/path";
import {
  PrintGoalPredicate as UnsafePrintGoalPredicate,
  PrintPredicateObligation as UnsafePrintPredicateObligation
} from "./private/predicate";
import { PrintTy as UnsafePrintTy } from "./private/ty";

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

export const PrintTy = ({ ty }: { ty: Ty }) => {
  return (
    <PrintWithFallback object={ty} Content={() => <UnsafePrintTy o={ty} />} />
  );
};

export const PrintObligation = ({ obligation }: { obligation: Obligation }) => {
  const InnerContent = () => (
    <ToggleGenericDelimiterContext.Provider value={true}>
      <UnsafePrintPredicateObligation o={obligation.obligation} />
    </ToggleGenericDelimiterContext.Provider>
  );
  return <PrintWithFallback object={obligation} Content={InnerContent} />;
};

export const PrintImplHeader = ({ impl }: { impl: ImplHeader }) => {
  return (
    <ToggleGenericDelimiterContext.Provider value={true}>
      <PrintWithFallback
        object={impl}
        Content={() => <UnsafePrintImplHeader o={impl} />}
      />
    </ToggleGenericDelimiterContext.Provider>
  );
};

export const PrintGoal = ({ o }: { o: GoalData }) => {
  const debugString =
    o.debugComparison === undefined ? null : (
      <div style={{ opacity: 0.5 }}>{o.debugComparison}</div>
    );
  const Content = () => (
    <ToggleGenericDelimiterContext.Provider value={true}>
      <UnsafePrintGoalPredicate o={o.value} />
      {debugString}
    </ToggleGenericDelimiterContext.Provider>
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

export const PrintBodyName = ({ defPath }: { defPath: DefinedPath }) => {
  return (
    <PrintWithFallback
      object={defPath}
      // Content={() => <UnsafePrintDefPathFull o={defPath} />}
      Content={() => <UnsafePrintDefPath o={defPath} />}
    />
  );
};
