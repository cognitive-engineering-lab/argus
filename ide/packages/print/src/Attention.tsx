import React, { useEffect, useRef } from "react";

import "./Attention.css";

const DURATION = 1_000;
const CN = "Attention";

const Attn = ({
  children,
  className = CN
}: React.PropsWithChildren<{ className?: string }>) => {
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    setTimeout(() => ref.current?.classList.remove(className), DURATION);
  }, []);

  return (
    <span ref={ref} className={className}>
      {children}
    </span>
  );
};

export const TextEmphasis = ({ children }: React.PropsWithChildren) => (
  <Attn className="AttentionText">{children}</Attn>
);

const Attention = ({ children }: React.PropsWithChildren) => (
  <Attn>{children}</Attn>
);

export default Attention;
