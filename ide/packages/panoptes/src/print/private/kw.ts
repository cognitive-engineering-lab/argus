import { Ident } from "@argus/common/bindings";

import "./kw.css";

// See https://doc.rust-lang.org/stable/nightly-rustc/rustc_span/symbol/kw/index.html

export const UnderscoreLifetime: Ident = { name: "'_" };

export const PathRoot: Ident = { name: "{{root}}" };
