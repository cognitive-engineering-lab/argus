import { Obligation } from "@argus/common/bindings";
import { Impl } from "@argus/common/bindings/serialization/hir/types";
import _ from "lodash";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";
import ReactJson from "react-json-view";

import "./print.css";
import { PrintImpl } from "./private/hir";
//@ts-ignore
import { PrintBinderPredicateKind } from "./private/predicate";

// NOTE: we only export the Obligation because that's all that's
// used within the obligations/tree view ATM. We wrap this in an
// error boundary to avoid any of the other untyped code from
// crashing the application.
export const PrettyObligation = ({
  obligation,
}: {
  obligation: Obligation;
}) => {
  console.log("Printing Obligation", obligation);
  const FallbackFromError = ErrorFactory(obligation);
  return (
    <ErrorBoundary
      FallbackComponent={FallbackFromError}
      onReset={details => {
        console.error(details);
      }}
    >
      <PrintBinderPredicateKind o={obligation.predicate} />
    </ErrorBoundary>
  );
};

const ErrorFactory = (o: Obligation) => {
  // TODO: allow resetting the error
  return ({
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
        <ReactJson src={o} />
        <pre>{error.message}</pre>
      </div>
    );
  };
};

export const PrintHirImplWithFallback = ({
  impl,
  fallback,
}: {
  impl: Impl;
  fallback: string;
}) => {
  const FallbackFromError = ({
    error,
    resetErrorBoundary,
  }: {
    error: any;
    resetErrorBoundary: (...args: any[]) => void;
  }) => {
    return (
      <div className="ImplFallback">
        <h1>Printing failed, fallback info</h1>

        <p>fallback</p>
      </div>
    );
  };
};
