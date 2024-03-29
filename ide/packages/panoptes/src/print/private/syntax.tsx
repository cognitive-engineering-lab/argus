import _ from "lodash";
import React from "react";

import "./syntax.css";

// See https://doc.rust-lang.org/stable/nightly-rustc/rustc_span/symbol/kw/index.html

// export const UnderscoreLifetime: Ident = { name: "'_" };

// export const PathRoot: Ident = { name: "{{root}}" };

export const Placeholder = ({ children }: React.PropsWithChildren) => {
  return <span className="placeholder">{children}</span>;
};

export const Kw = ({ children }: React.PropsWithChildren) => {
  return <span className="kw">{children}</span>;
};

export const Angled = ({ children }: React.PropsWithChildren) => {
  return (
    <span>
      {"<"}
      {children}
      {">"}
    </span>
  );
};

export const DBraced = ({ children }: React.PropsWithChildren) => {
  return (
    <span>
      {"{{"}
      {children}
      {"}}"}
    </span>
  );
};

export const CBraced = ({ children }: React.PropsWithChildren) => {
  return (
    <span>
      {"{"}
      {children}
      {"}"}
    </span>
  );
};

export const Parenthesized = ({ children }: React.PropsWithChildren) => {
  return (
    <span>
      {"("}
      {children}
      {")"}
    </span>
  );
};

export const SqBraced = ({ children }: React.PropsWithChildren) => {
  return (
    <span>
      {"["}
      {children}
      {"]"}
    </span>
  );
};

const Interspersed = ({
  components,
  sep,
}: {
  components: React.FC[];
  sep: string;
}) => {
  return _.map(components, (C, i) => {
    const p = i === 0 ? "" : sep;
    return (
      <span key={i}>
        {p}
        <C />
      </span>
    );
  });
};

export const CommaSeparated = ({ components }: { components: React.FC[] }) => {
  return <Interspersed components={components} sep=", " />;
};

export const PlusSeparated = ({ components }: { components: React.FC[] }) => {
  return <Interspersed components={components} sep=" + " />;
};
