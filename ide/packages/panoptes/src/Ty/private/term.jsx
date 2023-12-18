import React from "react";

import { PrintConst } from "./const";
import { PrintTy } from "./ty";

export const PrintTerm = ({ o }) => {
  if ("Ty" in o) {
    return <PrintTy o={o.Ty} />;
  } else if ("Const" in o) {
    return <PrintConst o={o.Const} />;
  } else {
    throw new Error("Unknown term", o);
  }
};
