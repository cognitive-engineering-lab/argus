import _ from "lodash";
import React from "react";

import "./syntax.css";
import classNames from "classnames";

export const nbsp = "\u00A0";

// A "Discretionary Space", the `inline-block` style helps format around these elements
// and breaks less between them and in random spaces.
export const Dsp = (
  props: React.PropsWithChildren & React.HTMLAttributes<HTMLElement>
) => {
  const kids = props.children;
  const htmlAttrs: React.HTMLAttributes<HTMLElement> = {
    ...props,
    children: undefined
  };
  return (
    <span style={{ display: "inline-block" }} {...htmlAttrs}>
      {kids}
    </span>
  );
};

/**
 * Highlight the children as placeholders, this means they aren't concrete types.
 *
 * For Argus, this usually means changing the foreground to something softer.
 */
export const Placeholder = ({ children }: React.PropsWithChildren) => (
  <span className="placeholder">{children}</span>
);

/**
 * Highlight the children as Rust keywords
 */
export const Kw = ({ children }: React.PropsWithChildren) => (
  <span className="kw">{children}</span>
);

/**
 * Create a wrapper around the children using a `stx-wrapper` class and the
 * additional class `c`. This makes a wrapper that breaks around the wrapped
 * elements.
 */
const makeCSSWrapper =
  (c: string) =>
  ({ children }: React.PropsWithChildren) => (
    <Dsp className={classNames("stx-wrapper", c)}>{children}</Dsp>
  );

/**
 * Create a wrapper that breaks around the children, but allows the `LHS` and `RHS`
 * wrapping elements to split from their children.
 */
const makeBreakingWrapper =
  (lhs: string, rhs: string) =>
  ({ children }: React.PropsWithChildren) => (
    <>
      {lhs}
      <Dsp>{children}</Dsp>
      {rhs}
    </>
  );

// We want content to break around parens and angle brackets.
// E.g., `fn foo<A,B>(a: A, b: B) -> B` could be formatted as:
// ```
// fn foo<
//   A, B
// >(
//   a: A,
//   b: B
// ) -> B
// ```
export const Angled = makeBreakingWrapper("<", ">");
export const Parenthesized = makeBreakingWrapper("(", ")");

export const DBraced = makeCSSWrapper("dbracket");
export const CBraced = makeCSSWrapper("bracket");
export const SqBraced = makeCSSWrapper("sqbracket");

export const CommaSeparated = ({
  components
}: { components: React.ReactElement[] }) => (
  <Interspersed components={components} sep="comma" />
);

export const PlusSeparated = ({
  components
}: { components: React.ReactElement[] }) => (
  <Interspersed components={components} sep="plus" />
);

const Interspersed = ({
  components,
  sep
}: {
  components: React.ReactElement[];
  sep: string;
}) => (
  <span className="interspersed-list">
    {_.map(components, (C, i) => (
      <Dsp key={i} className={sep}>
        {C}
      </Dsp>
    ))}
  </span>
);
