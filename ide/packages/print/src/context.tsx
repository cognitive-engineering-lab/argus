import type { DefinedPath, Ty, TyVal } from "@argus/common/bindings";
import { createContext } from "react";
import React, { type ReactElement } from "react";
import { PrintTyValue } from "./private/ty";

// Change this to true if we want to by default toggle type parameter lists
export const AllowToggle = createContext(false);
export const AllowPathTrim = createContext(true);
export const AllowProjectionSubst = createContext(true);

export interface TypeContext {
  interner: TyVal[];
  projections: Record<Ty, Ty>;
}

export const TyCtxt = createContext<TypeContext | undefined>(undefined);

export const DefPathRender = createContext(
  ({
    fullPath: _fp,
    ctx: _ctx,
    Head,
    Rest
  }: {
    ctx: TypeContext;
    fullPath: DefinedPath;
    Head: ReactElement;
    Rest: ReactElement;
  }) => (
    <>
      {Head}
      {Rest}
    </>
  )
);

export const ProjectionPathRender = createContext(
  ({
    original,
    projection: _prj,
    ctx: _ctx
  }: {
    ctx: TypeContext;
    original: TyVal;
    projection: TyVal;
  }) => <PrintTyValue o={original} />
);
