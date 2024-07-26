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

// -----------------------------------------
// Definition item options

export const DefinitionAction = createContext<
  (defId: DefinedPath) => React.MouseEventHandler
>(_d => () => null);

// -----------------------------------------
// Render options for a definition path

export type DefPathRenderProps = {
  ctx: TypeContext;
  fullPath: DefinedPath;
  Head: ReactElement;
  Rest: ReactElement;
};

export type DefPathRenderPropsKind = React.FC<DefPathRenderProps>;

export const DefPathRender = createContext<React.FC<DefPathRenderProps>>(
  ({ Head, Rest }) => (
    <>
      {Head}
      {Rest}
    </>
  )
);

// -----------------------------------------
// Render options for a type projection path

export type ProjectPathRenderProps = {
  ctx: TypeContext;
  original: TyVal;
  projection: TyVal;
};

export const ProjectionPathRender = createContext<
  React.FC<ProjectPathRenderProps>
>(({ original }) => <PrintTyValue o={original} />);
