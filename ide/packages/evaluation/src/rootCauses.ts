export interface RootCause {
  workspace: string;
  causes: FileCause[];
}

interface FileCause {
  file: string;
  message: string;
}

export const rootCauses: RootCause[] = [
  {
    workspace: "diesel",
    causes: [
      {
        file: "bad_sql_query.rs",
        message: "User: QueryableByName",
      },
      {
        file: "queryable_order_mismatch.rs",
        message: "(String, i32): FromSql",
      },
      {
        file: "invalid_select.rs",
        message: "id: SelectableExpression",
      },
      {
        file: "invalid_query.rs",
        message: "Count == Once",
      },
    ],
  },
  {
    workspace: "axum",
    causes: [
      {
        file: "argument_not_extractor.rs",
        message: "bool: FromRequestParts",
      },
      {
        file: "extract_self_mut.rs",
        message: "&'_ mut A: FromRequestParts",
      },
      {
        file: "extract_self_ref.rs",
        message: "&'_ A: FromRequestParts",
      },
      {
        file: "missing_deserialize.rs",
        message: "Test: Deserialize",
      },
      {
        file: "multiple_body_extractors.rs",
        message: "String: FromRequestParts",
      },
      {
        file: "request_not_last.rs",
        message: "Request<Body>: FromRequestParts",
      },
    ],
  },
  {
    workspace: "bevy",
    causes: [
      {
        file: "main.rs",
        message: "Timer: SystemParam",
      },
    ],
  },
  {
    workspace: "nalgebra",
    causes: [
      {
        file: "mat_mul.rs",
        message: "ShapeConstraint",
      },
    ],
  },
];
