import _ from "lodash";
import React from "react";

import "./syntax.css";

// A "Discretionary Space", hopefully this allows the layout to break along
// these elements rather than in the middle of text or random spaces.
export const Dsp = ({ children }: React.PropsWithChildren) => (
  <span style={{ display: "inline-block" }}>{children}</span>
);

export const Placeholder = ({ children }: React.PropsWithChildren) => (
  <span className="placeholder">{children}</span>
);

export const Kw = ({ children }: React.PropsWithChildren) => (
  <span className="kw">{children}</span>
);

const makeWrapper =
  (lhs: string, rhs: string) =>
  ({ children }: React.PropsWithChildren) => (
    <>
      {lhs}
      <Dsp>{children}</Dsp>
      {rhs}
    </>
  );

export const Angled = makeWrapper("<", ">");
export const DBraced = makeWrapper("{{", "}}");
export const CBraced = makeWrapper("{", "}");
export const Parenthesized = makeWrapper("(", ")");
export const SqBraced = makeWrapper("[", "]");

export const CommaSeparated = ({ components }: { components: React.FC[] }) => (
  <Interspersed components={components} sep=", " />
);

export const PlusSeparated = ({ components }: { components: React.FC[] }) => (
  <Interspersed components={components} sep=" + " />
);

const Interspersed = ({
  components,
  sep
}: {
  components: React.FC[];
  sep: string;
}) =>
  _.map(components, (C, i) => (
    // The inline-block span should help the layout to break on the elements
    // and not in them. Still undecided if this actually does anything.
    <React.Fragment key={i}>
      {i === 0 ? "" : sep}
      <span style={{ display: "inline-block" }}>
        <C />
      </span>
    </React.Fragment>
  ));
