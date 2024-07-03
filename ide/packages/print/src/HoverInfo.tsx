import {
  FloatingPortal,
  flip,
  offset,
  shift,
  useFloating,
  useHover,
  useInteractions
} from "@floating-ui/react";
import React, { useState } from "react";

import "./HoverInfo.css";

export const HoverInfo = ({
  Content,
  children
}: React.PropsWithChildren<{ Content: React.FC }>) => {
  const [isOpen, setIsOpen] = useState(false);
  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: setIsOpen,
    placement: "top",
    middleware: [offset(() => 5), flip(), shift()]
  });

  const hover = useHover(context, {
    delay: {
      open: 500
    }
  });

  const { getReferenceProps, getFloatingProps } = useInteractions([hover]);

  return (
    <>
      <div
        className="HoverMainInfo"
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        {children}
      </div>
      {isOpen && (
        <FloatingPortal>
          <div
            className="floating"
            ref={refs.setFloating}
            style={floatingStyles}
            {...getFloatingProps()}
          >
            <Content />
          </div>
        </FloatingPortal>
      )}
    </>
  );
};
