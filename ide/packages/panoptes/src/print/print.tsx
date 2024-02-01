import { Obligation } from "@argus/common/bindings";
import { Impl } from "@argus/common/bindings/serialization/hir/types";
import _ from "lodash";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";
import ReactJson from "react-json-view";

import "./print.css";
import { PrintImpl as UnsafePrintImpl } from "./private/hir";
import {
  PrintBinderPredicateKind,
  PrintGoalPredicate as UnsafePrintGoalPredicate,
} from "./private/predicate";
import { PrintTy as UnsafePrintTy } from "./private/ty";

// NOTE: please Please PLEASE wrap all printing components in this
// PrintWithFallback. Pretty printing is still a fragile process and
// I can never be sure if it's working correctly or not.
//
// Nothing should ever be imported from the 'private' directory except
// from within this file.
export const PrintWithFallback = ({
  object,
  Content,
}: {
  object: any;
  Content: React.FC;
}) => {
  const FallbackFromError = ({
    error,
    resetErrorBoundary,
  }: {
    error: any;
    resetErrorBoundary: (...args: any[]) => void;
  }) => {
    // NOTE: Call resetErrorBoundary() to reset the error boundary and retry the render.
    return (
      <div className="PrintError">
        <p>Whoops! Something went wrong while printing:</p>
        <ReactJson src={object} collapsed={true} />
        <pre>`{error.message}`</pre>
      </div>
    );
  };

  return (
    <ErrorBoundary
      FallbackComponent={FallbackFromError}
      onReset={details => {
        console.error(details);
      }}
    >
      <Content />
    </ErrorBoundary>
  );
};

export const PrintTy = ({ ty }: { ty: any }) => {
  return (
    <PrintWithFallback object={ty} Content={() => <UnsafePrintTy o={ty} />} />
  );
};

export const PrintObligation = ({ obligation }: { obligation: Obligation }) => {
  return (
    <PrintWithFallback
      object={obligation}
      Content={() => <PrintBinderPredicateKind o={obligation.predicate} />}
    />
  );
};

export const PrintImpl = ({ impl }: { impl: Impl }) => {
  return (
    <PrintWithFallback
      object={impl}
      Content={() => <UnsafePrintImpl impl={impl} />}
    />
  );
};

export const PrintGoal = ({ o }: { o: any }) => {
  return (
    <PrintWithFallback
      object={o}
      Content={() => <UnsafePrintGoalPredicate o={o} />}
    />
  );
};
