import _ from "lodash";
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
  // FIXME: this assumes that nobody is using a `font-size` smaller than 14.
  const fontSize = 14;
  return _.map(_.range(height / fontSize), i => <Spacer key={i} />);
};

export default FillScreen;
