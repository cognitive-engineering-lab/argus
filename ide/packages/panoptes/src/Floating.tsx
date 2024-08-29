import React, { useRef, useState } from "react";

import { composeEvents } from "@argus/common/func";
import {
  FloatingArrow,
  FloatingFocusManager,
  FloatingPortal,
  arrow,
  autoUpdate,
  flip,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions
} from "@floating-ui/react";
import classNames from "classnames";

import "./Floating.css";

export interface FloatingProps {
  toggle: React.JSX.Element;
  outerClassName?: string;
  innerClassName?: string;
  reportActive?: (b: boolean) => void;
}

const Floating = (props: React.PropsWithChildren<FloatingProps>) => {
  const [isOpen, setIsOpen] = useState(false);
  const openCallback = composeEvents(setIsOpen, props.reportActive);
  const arrowRef = useRef(null);

  const ARROW_HEIGHT = 10;
  const GAP = 5;

  const { refs, floatingStyles, context } = useFloating({
    open: isOpen,
    onOpenChange: openCallback,
    placement: "bottom",
    middleware: [
      offset(ARROW_HEIGHT + GAP),
      flip(),
      shift(),
      arrow({
        element: arrowRef
      })
    ],
    whileElementsMounted: autoUpdate
  });

  const click = useClick(context);
  const dismiss = useDismiss(context);
  const { getReferenceProps, getFloatingProps } = useInteractions([
    click,
    dismiss
  ]);

  return (
    <>
      <span
        className={props.outerClassName}
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        {props.toggle}
      </span>
      {isOpen && (
        <FloatingPortal>
          <FloatingFocusManager context={context}>
            <div
              className={classNames("floating", props.innerClassName)}
              ref={refs.setFloating}
              style={floatingStyles}
              {...getFloatingProps()}
              onClick={e => e.stopPropagation()}
            >
              <span>
                <FloatingArrow
                  className="floating-arrow"
                  ref={arrowRef}
                  context={context}
                  height={ARROW_HEIGHT}
                  tipRadius={3}
                  stroke="2"
                />
              </span>
              {props.children}
            </div>
          </FloatingFocusManager>
        </FloatingPortal>
      )}
    </>
  );
};

export default Floating;
