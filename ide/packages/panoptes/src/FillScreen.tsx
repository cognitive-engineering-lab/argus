import React, { useState, useEffect } from "react";

import "./FillScreen.css";

export function useWindowDimensions() {
  const initialDimensions = { width: 600, height: 800 };
  const [windowDimensions, setWindowDimensions] = useState(initialDimensions);

  const getWindowDimensions = () => {
    if (typeof window === "undefined") return initialDimensions;
    return {
      width: document.documentElement.clientWidth,
      height: document.documentElement.clientHeight
    };
  };

  useEffect(() => {
    const handleResize = () => {
      setWindowDimensions(getWindowDimensions());
    };

    if (typeof window !== "undefined") {
      window.addEventListener("resize", handleResize);
      return () => window.removeEventListener("resize", handleResize);
    }
  }, []);

  return windowDimensions;
}

export const Spacer = () => <div className="spacer">{"\u00A0"}</div>;

const FillScreen = () => {
  const { height } = useWindowDimensions();
  // 75% of the screen height means there is always *something* visible,
  // and the user can almost scroll the contents to the top of the view.
  return <div style={{ height: height * 0.75, width: "100%" }} />;
};

export default FillScreen;
