import _ from "lodash";
import React from "react";
import { ErrorBoundary } from "react-error-boundary";

import "./print.css";
import { PrintBinderPredicateKind } from "./private/predicate";

// NOTE: we only export the Obligation because that's all that's
// used within the obligations/tree view ATM. We wrap this in an
// error boundary to avoid any of the other untyped code from
// crashing the application.
export const PrettyObligation = ({ obligation }) => {
  console.log("Printing Obligation", obligation);
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

const FallbackFromError = ({ error, resetErrorBoundary }) => {
  // NOTE: Call resetErrorBoundary() to reset the error boundary and retry the render.
  return (
    <div className="PrintError">
      <p>Whoops! Something went wrong:</p>
      <pre>{error.message}</pre>
    </div>
  );
};
