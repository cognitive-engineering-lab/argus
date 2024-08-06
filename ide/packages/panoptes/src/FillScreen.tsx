import React, { useState, useEffect } from "react";

import "./FillScreen.css";

function getWindowDimensions() {
  const { innerWidth: width, innerHeight: height } = window;
  return {
    width,
    height
  };
}

function useWindowDimensions() {
  const [windowDimensions, setWindowDimensions] = useState(
    getWindowDimensions()
  );

  useEffect(() => {
    function handleResize() {
      setWindowDimensions(getWindowDimensions());
    }

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
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
