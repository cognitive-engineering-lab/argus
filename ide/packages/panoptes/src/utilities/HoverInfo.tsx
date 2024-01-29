import { useFloating, useHover, useInteractions } from "@floating-ui/react";
import _ from "lodash";
import React, { useState } from "react";

import "./HoverInfo.css";

export const HoverInfo = ({
  content,
  children,
}: React.PropsWithChildren<{ content: React.ReactElement }>) => {
  const [isOpen, setIsOpen] = useState(false);
  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: setIsOpen,
  });
  const hover = useHover(context);
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
        <div
          className="floating"
          ref={refs.setFloating}
          style={floatingStyles}
          {...getFloatingProps}
        >
          {content}
        </div>
      )}
    </>
  );
};
