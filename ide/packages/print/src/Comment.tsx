import {
  FloatingFocusManager,
  FloatingPortal,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions
} from "@floating-ui/react";
import classNames from "classnames";
import React, { useState } from "react";
import { IcoComment } from "./Icons";

const Comment = ({
  Child,
  Content
}: {
  Child: React.ReactElement;
  Content: React.ReactElement;
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: setIsOpen,
    placement: "bottom",
    middleware: [offset(() => 5), shift()]
  });

  const click = useClick(context);
  const dismiss = useDismiss(context);
  const { getReferenceProps, getFloatingProps } = useInteractions([
    click,
    dismiss
  ]);

  return (
    <>
      {Child}
      <span
        className="tree-toggle"
        ref={refs.setReference}
        style={{ verticalAlign: "super", fontSize: "0.4rem" }}
        {...getReferenceProps()}
      >
        <IcoComment />
      </span>
      {isOpen && (
        <FloatingPortal>
          <FloatingFocusManager context={context}>
            <div
              className={classNames("floating", "floating-graph")}
              ref={refs.setFloating}
              style={floatingStyles}
              {...getFloatingProps()}
            >
              <div style={{ padding: "0.25em" }}>{Content}</div>
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
};

export default Comment;
