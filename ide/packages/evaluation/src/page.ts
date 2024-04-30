import { BodyBundle } from "@argus/common/bindings";
import {
  ConfigConsts,
  ErrorJumpTargetInfo,
  EvaluationMode,
  Filename,
  PanoptesConfig,
  configToString,
} from "@argus/common/lib";
import _ from "lodash";
import path from "path";

import { PORT } from "./serve";

// Default VSCode Light styles
export const defaultStyles = `
--background: #ffffff;
--contrast-active-border: transparent;
--focus-border: #0090f1;
--font-family: -apple-system, BlinkMacSystemFont, sans-serif;
--font-weight: normal;
--foreground: #616161;
--scrollbar-slider-background: rgba(100, 100, 100, 0.4);
--scrollbar-slider-hover-background: rgba(100, 100, 100, 0.7);
--scrollbar-slider-active-background: rgba(0, 0, 0, 0.6);
--badge-background: #c4c4c4;
--badge-foreground: #333333;
--button-primary-background: #007acc;
--button-primary-hover-background: #0062a3;
--button-secondary-background: #5f6a79;
--button-secondary-hover-background: #4c5561;
--checkbox-background: #ffffff;
--checkbox-border: #919191;
--checkbox-foreground: #616161;
--list-active-selection-background: #0060c0;
--list-hover-background: #e8e8e8;
--divider-background: #c8c8c8;
--dropdown-background: #ffffff;
--dropdown-border: #cecece;
--dropdown-foreground: #616161;
--input-background: #ffffff;
--input-foreground: #616161;
--input-placeholder-foreground: #767676;
--link-active-foreground: #006ab1;
--link-foreground: #006ab1;
--panel-tab-active-border: #424242;
--panel-tab-active-foreground: #424242;
--panel-tab-foreground: rgba(66, 66, 66, 0.75);
--panel-view-background: #ffffff;
--panel-view-border: rgba(128, 128, 128, 0.35);

--vscode-foreground: #616161;
--vscode-disabledForeground: rgba(97, 97, 97, 0.5);
--vscode-errorForeground: #a1260d;
--vscode-descriptionForeground: #717171;
--vscode-icon-foreground: #424242;
--vscode-focusBorder: #0090f1;
--vscode-textSeparator-foreground: rgba(0, 0, 0, 0.18);
--vscode-textLink-foreground: #006ab1;
--vscode-textLink-activeForeground: #006ab1;
--vscode-textPreformat-foreground: #a31515;
--vscode-textPreformat-background: rgba(0, 0, 0, 0.1);
--vscode-textBlockQuote-background: #f2f2f2;
--vscode-textBlockQuote-border: rgba(0, 122, 204, 0.5);
--vscode-textCodeBlock-background: rgba(220, 220, 220, 0.4);
--vscode-widget-shadow: rgba(0, 0, 0, 0.16);
--vscode-widget-border: #d4d4d4;
--vscode-input-background: #ffffff;
--vscode-input-foreground: #616161;
--vscode-inputOption-activeBorder: #007acc;
--vscode-inputOption-hoverBackground: rgba(184, 184, 184, 0.31);
--vscode-inputOption-activeBackground: rgba(0, 144, 241, 0.2);
--vscode-inputOption-activeForeground: #000000;
--vscode-input-placeholderForeground: #767676;
--vscode-inputValidation-infoBackground: #d6ecf2;
--vscode-inputValidation-infoBorder: #007acc;
--vscode-inputValidation-warningBackground: #f6f5d2;
--vscode-inputValidation-warningBorder: #b89500;
--vscode-inputValidation-errorBackground: #f2dede;
--vscode-inputValidation-errorBorder: #be1100;
--vscode-dropdown-background: #ffffff;
--vscode-dropdown-foreground: #616161;
--vscode-dropdown-border: #cecece;
--vscode-button-foreground: #ffffff;
--vscode-button-separator: rgba(255, 255, 255, 0.4);
--vscode-button-background: #007acc;
--vscode-button-hoverBackground: #0062a3;
--vscode-button-secondaryForeground: #ffffff;
--vscode-button-secondaryBackground: #5f6a79;
--vscode-button-secondaryHoverBackground: #4c5561;
--vscode-badge-background: #c4c4c4;
--vscode-badge-foreground: #333333;
--vscode-scrollbar-shadow: #dddddd;
--vscode-scrollbarSlider-background: rgba(100, 100, 100, 0.4);
--vscode-scrollbarSlider-hoverBackground: rgba(100, 100, 100, 0.7);
--vscode-scrollbarSlider-activeBackground: rgba(0, 0, 0, 0.6);
--vscode-progressBar-background: #0e70c0;
--vscode-editorError-foreground: #e51400;
--vscode-editorWarning-foreground: #bf8803;
--vscode-editorInfo-foreground: #1a85ff;
--vscode-editorHint-foreground: #6c6c6c;
--vscode-sash-hoverBorder: #0090f1;
--vscode-editor-background: #ffffff;
--vscode-editor-foreground: #000000;
--vscode-editorStickyScroll-background: #ffffff;
--vscode-editorStickyScrollHover-background: #f0f0f0;
--vscode-editorStickyScroll-shadow: #dddddd;
--vscode-editorWidget-background: #f3f3f3;
--vscode-editorWidget-foreground: #616161;
--vscode-editorWidget-border: #c8c8c8;
--vscode-quickInput-background: #f3f3f3;
--vscode-quickInput-foreground: #616161;
--vscode-quickInputTitle-background: rgba(0, 0, 0, 0.06);
--vscode-pickerGroup-foreground: #0066bf;
--vscode-pickerGroup-border: #cccedb;
--vscode-keybindingLabel-background: rgba(221, 221, 221, 0.4);
--vscode-keybindingLabel-foreground: #555555;
--vscode-keybindingLabel-border: rgba(204, 204, 204, 0.4);
--vscode-keybindingLabel-bottomBorder: rgba(187, 187, 187, 0.4);
--vscode-editor-selectionBackground: #add6ff;
--vscode-editor-inactiveSelectionBackground: #e5ebf1;
--vscode-editor-selectionHighlightBackground: rgba(173, 214, 255, 0.5);
--vscode-editor-findMatchBackground: #a8ac94;
--vscode-editor-findMatchHighlightBackground: rgba(234, 92, 0, 0.33);
--vscode-editor-findRangeHighlightBackground: rgba(180, 180, 180, 0.3);
--vscode-searchEditor-findMatchBackground: rgba(234, 92, 0, 0.22);
--vscode-search-resultsInfoForeground: #616161;
--vscode-editor-hoverHighlightBackground: rgba(173, 214, 255, 0.15);
--vscode-editorHoverWidget-background: #f3f3f3;
--vscode-editorHoverWidget-foreground: #616161;
--vscode-editorHoverWidget-border: #c8c8c8;
--vscode-editorHoverWidget-statusBarBackground: #e7e7e7;
--vscode-editorLink-activeForeground: #0000ff;
--vscode-editorInlayHint-foreground: #969696;
--vscode-editorInlayHint-background: rgba(196, 196, 196, 0.1);
--vscode-editorInlayHint-typeForeground: #969696;
--vscode-editorInlayHint-typeBackground: rgba(196, 196, 196, 0.1);
--vscode-editorInlayHint-parameterForeground: #969696;
--vscode-editorInlayHint-parameterBackground: rgba(196, 196, 196, 0.1);
--vscode-editorLightBulb-foreground: #ddb100;
--vscode-editorLightBulbAutoFix-foreground: #007acc;
--vscode-editorLightBulbAi-foreground: #ddb100;
--vscode-diffEditor-insertedTextBackground: rgba(156, 204, 44, 0.25);
--vscode-diffEditor-removedTextBackground: rgba(255, 0, 0, 0.2);
--vscode-diffEditor-insertedLineBackground: rgba(155, 185, 85, 0.2);
--vscode-diffEditor-removedLineBackground: rgba(255, 0, 0, 0.2);
--vscode-diffEditor-diagonalFill: rgba(34, 34, 34, 0.2);
--vscode-diffEditor-unchangedRegionBackground: #f8f8f8;
--vscode-diffEditor-unchangedRegionForeground: #616161;
--vscode-diffEditor-unchangedCodeBackground: rgba(184, 184, 184, 0.16);
--vscode-list-focusOutline: #0090f1;
--vscode-list-focusAndSelectionOutline: #90c2f9;
--vscode-list-activeSelectionBackground: #0060c0;
--vscode-list-activeSelectionForeground: #ffffff;
--vscode-list-activeSelectionIconForeground: #ffffff;
--vscode-list-inactiveSelectionBackground: #e4e6f1;
--vscode-list-hoverBackground: #e8e8e8;
--vscode-list-dropBackground: #d6ebff;
--vscode-list-dropBetweenBackground: #424242;
--vscode-list-highlightForeground: #0066bf;
--vscode-list-focusHighlightForeground: #bbe7ff;
--vscode-list-invalidItemForeground: #b89500;
--vscode-list-errorForeground: #b01011;
--vscode-list-warningForeground: #855f00;
--vscode-listFilterWidget-background: #f3f3f3;
--vscode-listFilterWidget-outline: rgba(0, 0, 0, 0);
--vscode-listFilterWidget-noMatchesOutline: #be1100;
--vscode-listFilterWidget-shadow: rgba(0, 0, 0, 0.16);
--vscode-list-filterMatchBackground: rgba(234, 92, 0, 0.33);
--vscode-tree-indentGuidesStroke: #a9a9a9;
--vscode-tree-inactiveIndentGuidesStroke: rgba(169, 169, 169, 0.4);
--vscode-tree-tableColumnsBorder: rgba(97, 97, 97, 0.13);
--vscode-tree-tableOddRowsBackground: rgba(97, 97, 97, 0.04);
--vscode-list-deemphasizedForeground: #8e8e90;
--vscode-checkbox-background: #ffffff;
--vscode-checkbox-selectBackground: #f3f3f3;
--vscode-checkbox-foreground: #616161;
--vscode-checkbox-border: #919191;
--vscode-checkbox-selectBorder: #424242;
--vscode-quickInputList-focusForeground: #ffffff;
--vscode-quickInputList-focusIconForeground: #ffffff;
--vscode-quickInputList-focusBackground: #0060c0;
--vscode-menu-border: #d4d4d4;
--vscode-menu-foreground: #616161;
--vscode-menu-background: #ffffff;
--vscode-menu-selectionForeground: #ffffff;
--vscode-menu-selectionBackground: #0060c0;
--vscode-menu-separatorBackground: #d4d4d4;
--vscode-toolbar-hoverBackground: rgba(184, 184, 184, 0.31);
--vscode-toolbar-activeBackground: rgba(166, 166, 166, 0.31);
--vscode-editor-snippetTabstopHighlightBackground: rgba(10, 50, 100, 0.2);
--vscode-editor-snippetFinalTabstopHighlightBorder: rgba(10, 50, 100, 0.5);
--vscode-breadcrumb-foreground: rgba(97, 97, 97, 0.8);
--vscode-breadcrumb-background: #ffffff;
--vscode-breadcrumb-focusForeground: #4e4e4e;
--vscode-breadcrumb-activeSelectionForeground: #4e4e4e;
--vscode-breadcrumbPicker-background: #f3f3f3;
--vscode-merge-currentHeaderBackground: rgba(64, 200, 174, 0.5);
--vscode-merge-currentContentBackground: rgba(64, 200, 174, 0.2);
--vscode-merge-incomingHeaderBackground: rgba(64, 166, 255, 0.5);
--vscode-merge-incomingContentBackground: rgba(64, 166, 255, 0.2);
--vscode-merge-commonHeaderBackground: rgba(96, 96, 96, 0.4);
--vscode-merge-commonContentBackground: rgba(96, 96, 96, 0.16);
--vscode-editorOverviewRuler-currentContentForeground: rgba(64, 200, 174, 0.5);
--vscode-editorOverviewRuler-incomingContentForeground: rgba(64, 166, 255, 0.5);
--vscode-editorOverviewRuler-commonContentForeground: rgba(96, 96, 96, 0.4);
--vscode-editorOverviewRuler-findMatchForeground: rgba(209, 134, 22, 0.49);
--vscode-editorOverviewRuler-selectionHighlightForeground: rgba(160, 160, 160, 0.8);
--vscode-minimap-findMatchHighlight: #d18616;
--vscode-minimap-selectionOccurrenceHighlight: #c9c9c9;
--vscode-minimap-selectionHighlight: #add6ff;
--vscode-minimap-infoHighlight: #1a85ff;
--vscode-minimap-warningHighlight: #bf8803;
--vscode-minimap-errorHighlight: rgba(255, 18, 18, 0.7);
--vscode-minimap-foregroundOpacity: #000000;
--vscode-minimapSlider-background: rgba(100, 100, 100, 0.2);
--vscode-minimapSlider-hoverBackground: rgba(100, 100, 100, 0.35);
--vscode-minimapSlider-activeBackground: rgba(0, 0, 0, 0.3);
--vscode-problemsErrorIcon-foreground: #e51400;
--vscode-problemsWarningIcon-foreground: #bf8803;
--vscode-problemsInfoIcon-foreground: #1a85ff;
--vscode-charts-foreground: #616161;
--vscode-charts-lines: rgba(97, 97, 97, 0.5);
--vscode-charts-red: #e51400;
--vscode-charts-blue: #1a85ff;
--vscode-charts-yellow: #bf8803;
--vscode-charts-orange: #d18616;
--vscode-charts-green: #388a34;
--vscode-charts-purple: #652d90;
--vscode-multiDiffEditor-headerBackground: #ececec;
--vscode-multiDiffEditor-border: #cccccc;
--vscode-symbolIcon-arrayForeground: #616161;
--vscode-symbolIcon-booleanForeground: #616161;
--vscode-symbolIcon-classForeground: #d67e00;
--vscode-symbolIcon-colorForeground: #616161;
--vscode-symbolIcon-constantForeground: #616161;
--vscode-symbolIcon-constructorForeground: #652d90;
--vscode-symbolIcon-enumeratorForeground: #d67e00;
--vscode-symbolIcon-enumeratorMemberForeground: #007acc;
--vscode-symbolIcon-eventForeground: #d67e00;
--vscode-symbolIcon-fieldForeground: #007acc;
--vscode-symbolIcon-fileForeground: #616161;
--vscode-symbolIcon-folderForeground: #616161;
--vscode-symbolIcon-functionForeground: #652d90;
--vscode-symbolIcon-interfaceForeground: #007acc;
--vscode-symbolIcon-keyForeground: #616161;
--vscode-symbolIcon-keywordForeground: #616161;
--vscode-symbolIcon-methodForeground: #652d90;
--vscode-symbolIcon-moduleForeground: #616161;
--vscode-symbolIcon-namespaceForeground: #616161;
--vscode-symbolIcon-nullForeground: #616161;
--vscode-symbolIcon-numberForeground: #616161;
--vscode-symbolIcon-objectForeground: #616161;
--vscode-symbolIcon-operatorForeground: #616161;
--vscode-symbolIcon-packageForeground: #616161;
--vscode-symbolIcon-propertyForeground: #616161;
--vscode-symbolIcon-referenceForeground: #616161;
--vscode-symbolIcon-snippetForeground: #616161;
--vscode-symbolIcon-stringForeground: #616161;
--vscode-symbolIcon-structForeground: #616161;
--vscode-symbolIcon-textForeground: #616161;
--vscode-symbolIcon-typeParameterForeground: #616161;
--vscode-symbolIcon-unitForeground: #616161;
--vscode-symbolIcon-variableForeground: #007acc;
--vscode-actionBar-toggledBackground: #dddddd;
--vscode-editorHoverWidget-highlightForeground: #0066bf;
--vscode-editor-lineHighlightBorder: #eeeeee;
--vscode-editor-rangeHighlightBackground: rgba(253, 255, 0, 0.2);
--vscode-editor-symbolHighlightBackground: rgba(234, 92, 0, 0.33);
--vscode-editorCursor-foreground: #000000;
--vscode-editorWhitespace-foreground: rgba(51, 51, 51, 0.2);
--vscode-editorLineNumber-foreground: #237893;
--vscode-editorIndentGuide-background: rgba(51, 51, 51, 0.2);
--vscode-editorIndentGuide-activeBackground: rgba(51, 51, 51, 0.2);
--vscode-editorIndentGuide-background1: #d3d3d3;
--vscode-editorIndentGuide-background2: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-background3: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-background4: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-background5: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-background6: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-activeBackground1: #939393;
--vscode-editorIndentGuide-activeBackground2: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-activeBackground3: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-activeBackground4: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-activeBackground5: rgba(0, 0, 0, 0);
--vscode-editorIndentGuide-activeBackground6: rgba(0, 0, 0, 0);
--vscode-editorActiveLineNumber-foreground: #0b216f;
--vscode-editorLineNumber-activeForeground: #0b216f;
--vscode-editorRuler-foreground: #d3d3d3;
--vscode-editorCodeLens-foreground: #919191;
--vscode-editorBracketMatch-background: rgba(0, 100, 0, 0.1);
--vscode-editorBracketMatch-border: #b9b9b9;
--vscode-editorOverviewRuler-border: rgba(127, 127, 127, 0.3);
--vscode-editorGutter-background: #ffffff;
--vscode-editorUnnecessaryCode-opacity: rgba(0, 0, 0, 0.47);
--vscode-editorGhostText-foreground: rgba(0, 0, 0, 0.47);
--vscode-editorOverviewRuler-rangeHighlightForeground: rgba(0, 122, 204, 0.6);
--vscode-editorOverviewRuler-errorForeground: rgba(255, 18, 18, 0.7);
--vscode-editorOverviewRuler-warningForeground: #bf8803;
--vscode-editorOverviewRuler-infoForeground: #1a85ff;
--vscode-editorBracketHighlight-foreground1: #0431fa;
--vscode-editorBracketHighlight-foreground2: #319331;
--vscode-editorBracketHighlight-foreground3: #7b3814;
--vscode-editorBracketHighlight-foreground4: rgba(0, 0, 0, 0);
--vscode-editorBracketHighlight-foreground5: rgba(0, 0, 0, 0);
--vscode-editorBracketHighlight-foreground6: rgba(0, 0, 0, 0);
--vscode-editorBracketHighlight-unexpectedBracket-foreground: rgba(255, 18, 18, 0.8);
--vscode-editorBracketPairGuide-background1: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-background2: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-background3: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-background4: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-background5: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-background6: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-activeBackground1: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-activeBackground2: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-activeBackground3: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-activeBackground4: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-activeBackground5: rgba(0, 0, 0, 0);
--vscode-editorBracketPairGuide-activeBackground6: rgba(0, 0, 0, 0);
--vscode-editorUnicodeHighlight-border: #bf8803;
--vscode-diffEditor-move-border: rgba(139, 139, 139, 0.61);
--vscode-diffEditor-moveActive-border: #ffa500;
--vscode-diffEditor-unchangedRegionShadow: rgba(115, 115, 115, 0.75);
--vscode-editorOverviewRuler-bracketMatchForeground: #a0a0a0;
--vscode-editor-foldBackground: rgba(173, 214, 255, 0.3);
--vscode-editorGutter-foldingControlForeground: #424242;
--vscode-editor-linkedEditingBackground: rgba(255, 0, 0, 0.3);
--vscode-editor-wordHighlightBackground: rgba(87, 87, 87, 0.25);
--vscode-editor-wordHighlightStrongBackground: rgba(14, 99, 156, 0.25);
--vscode-editor-wordHighlightTextBackground: rgba(87, 87, 87, 0.25);
--vscode-editorOverviewRuler-wordHighlightForeground: rgba(160, 160, 160, 0.8);
--vscode-editorOverviewRuler-wordHighlightStrongForeground: rgba(192, 160, 192, 0.8);
--vscode-editorOverviewRuler-wordHighlightTextForeground: rgba(160, 160, 160, 0.8);
--vscode-peekViewTitle-background: #f3f3f3;
--vscode-peekViewTitleLabel-foreground: #000000;
--vscode-peekViewTitleDescription-foreground: #616161;
--vscode-peekView-border: #1a85ff;
--vscode-peekViewResult-background: #f3f3f3;
--vscode-peekViewResult-lineForeground: #646465;
--vscode-peekViewResult-fileForeground: #1e1e1e;
--vscode-peekViewResult-selectionBackground: rgba(51, 153, 255, 0.2);
--vscode-peekViewResult-selectionForeground: #6c6c6c;
--vscode-peekViewEditor-background: #f2f8fc;
--vscode-peekViewEditorGutter-background: #f2f8fc;
--vscode-peekViewEditorStickyScroll-background: #f2f8fc;
--vscode-peekViewResult-matchHighlightBackground: rgba(234, 92, 0, 0.3);
--vscode-peekViewEditor-matchHighlightBackground: rgba(245, 216, 2, 0.87);
--vscode-editorMarkerNavigationError-background: #e51400;
--vscode-editorMarkerNavigationError-headerBackground: rgba(229, 20, 0, 0.1);
--vscode-editorMarkerNavigationWarning-background: #bf8803;
--vscode-editorMarkerNavigationWarning-headerBackground: rgba(191, 136, 3, 0.1);
--vscode-editorMarkerNavigationInfo-background: #1a85ff;
--vscode-editorMarkerNavigationInfo-headerBackground: rgba(26, 133, 255, 0.1);
--vscode-editorMarkerNavigation-background: #ffffff;
--vscode-editorSuggestWidget-background: #f3f3f3;
--vscode-editorSuggestWidget-border: #c8c8c8;
--vscode-editorSuggestWidget-foreground: #000000;
--vscode-editorSuggestWidget-selectedForeground: #ffffff;
--vscode-editorSuggestWidget-selectedIconForeground: #ffffff;
--vscode-editorSuggestWidget-selectedBackground: #0060c0;
--vscode-editorSuggestWidget-highlightForeground: #0066bf;
--vscode-editorSuggestWidget-focusHighlightForeground: #bbe7ff;
--vscode-editorSuggestWidgetStatus-foreground: rgba(0, 0, 0, 0.5);
--vscode-editorWatermark-foreground: rgba(0, 0, 0, 0.68);
--vscode-tab-activeBackground: #ffffff;
--vscode-tab-unfocusedActiveBackground: #ffffff;
--vscode-tab-inactiveBackground: #ececec;
--vscode-tab-unfocusedInactiveBackground: #ececec;
--vscode-tab-activeForeground: #333333;
--vscode-tab-inactiveForeground: rgba(51, 51, 51, 0.7);
--vscode-tab-unfocusedActiveForeground: rgba(51, 51, 51, 0.7);
--vscode-tab-unfocusedInactiveForeground: rgba(51, 51, 51, 0.35);
--vscode-tab-border: #f3f3f3;
--vscode-tab-lastPinnedBorder: rgba(97, 97, 97, 0.19);
--vscode-tab-dragAndDropBorder: #333333;
--vscode-tab-activeModifiedBorder: #33aaee;
--vscode-tab-inactiveModifiedBorder: rgba(51, 170, 238, 0.5);
--vscode-tab-unfocusedActiveModifiedBorder: rgba(51, 170, 238, 0.7);
--vscode-tab-unfocusedInactiveModifiedBorder: rgba(51, 170, 238, 0.25);
--vscode-editorPane-background: #ffffff;
--vscode-editorGroupHeader-tabsBackground: #f3f3f3;
--vscode-editorGroupHeader-noTabsBackground: #ffffff;
--vscode-editorGroup-border: #e7e7e7;
--vscode-editorGroup-dropBackground: rgba(38, 119, 203, 0.18);
--vscode-editorGroup-dropIntoPromptForeground: #616161;
--vscode-editorGroup-dropIntoPromptBackground: #f3f3f3;
--vscode-sideBySideEditor-horizontalBorder: #e7e7e7;
--vscode-sideBySideEditor-verticalBorder: #e7e7e7;
--vscode-panel-background: #ffffff;
--vscode-panel-border: rgba(128, 128, 128, 0.35);
--vscode-panelTitle-activeForeground: #424242;
--vscode-panelTitle-inactiveForeground: rgba(66, 66, 66, 0.75);
--vscode-panelTitle-activeBorder: #424242;
--vscode-panelInput-border: #dddddd;
--vscode-panel-dropBorder: #424242;
--vscode-panelSection-dropBackground: rgba(38, 119, 203, 0.18);
--vscode-panelSectionHeader-background: rgba(128, 128, 128, 0.2);
--vscode-panelSection-border: rgba(128, 128, 128, 0.35);
--vscode-banner-background: #004386;
--vscode-banner-foreground: #ffffff;
--vscode-banner-iconForeground: #1a85ff;
--vscode-statusBar-foreground: #ffffff;
--vscode-statusBar-noFolderForeground: #ffffff;
--vscode-statusBar-background: #007acc;
--vscode-statusBar-noFolderBackground: #68217a;
--vscode-statusBar-focusBorder: #ffffff;
--vscode-statusBarItem-activeBackground: rgba(255, 255, 255, 0.18);
--vscode-statusBarItem-focusBorder: #ffffff;
--vscode-statusBarItem-hoverBackground: rgba(255, 255, 255, 0.12);
--vscode-statusBarItem-hoverForeground: #ffffff;
--vscode-statusBarItem-compactHoverBackground: rgba(255, 255, 255, 0.2);
--vscode-statusBarItem-prominentForeground: #ffffff;
--vscode-statusBarItem-prominentBackground: rgba(0, 0, 0, 0.5);
--vscode-statusBarItem-prominentHoverForeground: #ffffff;
--vscode-statusBarItem-prominentHoverBackground: rgba(0, 0, 0, 0.3);
--vscode-statusBarItem-errorBackground: #c72e0f;
--vscode-statusBarItem-errorForeground: #ffffff;
--vscode-statusBarItem-errorHoverForeground: #ffffff;
--vscode-statusBarItem-errorHoverBackground: rgba(255, 255, 255, 0.12);
--vscode-statusBarItem-warningBackground: #725102;
--vscode-statusBarItem-warningForeground: #ffffff;
--vscode-statusBarItem-warningHoverForeground: #ffffff;
--vscode-statusBarItem-warningHoverBackground: rgba(255, 255, 255, 0.12);
--vscode-activityBar-background: #2c2c2c;
--vscode-activityBar-foreground: #ffffff;
--vscode-activityBar-inactiveForeground: rgba(255, 255, 255, 0.4);
--vscode-activityBar-activeBorder: #ffffff;
--vscode-activityBar-dropBorder: #ffffff;
--vscode-activityBarBadge-background: #007acc;
--vscode-activityBarBadge-foreground: #ffffff;
--vscode-activityBarTop-foreground: #424242;
--vscode-activityBarTop-activeBorder: #424242;
--vscode-activityBarTop-inactiveForeground: rgba(66, 66, 66, 0.75);
--vscode-activityBarTop-dropBorder: #424242;
--vscode-profileBadge-background: #c4c4c4;
--vscode-profileBadge-foreground: #333333;
--vscode-statusBarItem-remoteBackground: #16825d;
--vscode-statusBarItem-remoteForeground: #ffffff;
--vscode-statusBarItem-remoteHoverForeground: #ffffff;
--vscode-statusBarItem-remoteHoverBackground: rgba(255, 255, 255, 0.12);
--vscode-statusBarItem-offlineBackground: #6c1717;
--vscode-statusBarItem-offlineForeground: #ffffff;
--vscode-statusBarItem-offlineHoverForeground: #ffffff;
--vscode-statusBarItem-offlineHoverBackground: rgba(255, 255, 255, 0.12);
--vscode-extensionBadge-remoteBackground: #007acc;
--vscode-extensionBadge-remoteForeground: #ffffff;
--vscode-sideBar-background: #f3f3f3;
--vscode-sideBarTitle-foreground: #6f6f6f;
--vscode-sideBar-dropBackground: rgba(38, 119, 203, 0.18);
--vscode-sideBarSectionHeader-background: rgba(0, 0, 0, 0);
--vscode-sideBarSectionHeader-border: rgba(97, 97, 97, 0.19);
--vscode-titleBar-activeForeground: #333333;
--vscode-titleBar-inactiveForeground: rgba(51, 51, 51, 0.6);
--vscode-titleBar-activeBackground: #dddddd;
--vscode-titleBar-inactiveBackground: rgba(221, 221, 221, 0.6);
--vscode-menubar-selectionForeground: #333333;
--vscode-menubar-selectionBackground: rgba(184, 184, 184, 0.31);
--vscode-commandCenter-foreground: #333333;
--vscode-commandCenter-activeForeground: #333333;
--vscode-commandCenter-inactiveForeground: rgba(51, 51, 51, 0.6);
--vscode-commandCenter-background: rgba(0, 0, 0, 0.05);
--vscode-commandCenter-activeBackground: rgba(0, 0, 0, 0.08);
--vscode-commandCenter-border: rgba(51, 51, 51, 0.2);
--vscode-commandCenter-activeBorder: rgba(51, 51, 51, 0.3);
--vscode-commandCenter-inactiveBorder: rgba(51, 51, 51, 0.15);
--vscode-notificationCenter-border: #d4d4d4;
--vscode-notificationToast-border: #d4d4d4;
--vscode-notifications-foreground: #616161;
--vscode-notifications-background: #f3f3f3;
--vscode-notificationLink-foreground: #006ab1;
--vscode-notificationCenterHeader-background: #e7e7e7;
--vscode-notifications-border: #e7e7e7;
--vscode-notificationsErrorIcon-foreground: #e51400;
--vscode-notificationsWarningIcon-foreground: #bf8803;
--vscode-notificationsInfoIcon-foreground: #1a85ff;
--vscode-chat-requestBorder: rgba(0, 0, 0, 0.1);
--vscode-chat-requestBackground: rgba(255, 255, 255, 0.62);
--vscode-chat-slashCommandBackground: #d2ecff;
--vscode-chat-slashCommandForeground: #306ca2;
--vscode-chat-avatarBackground: #f2f2f2;
--vscode-chat-avatarForeground: #616161;
--vscode-simpleFindWidget-sashBorder: #c8c8c8;
--vscode-commentsView-resolvedIcon: rgba(97, 97, 97, 0.5);
--vscode-commentsView-unresolvedIcon: #0090f1;
--vscode-editorCommentsWidget-replyInputBackground: #f3f3f3;
--vscode-editorCommentsWidget-resolvedBorder: rgba(97, 97, 97, 0.5);
--vscode-editorCommentsWidget-unresolvedBorder: #0090f1;
--vscode-editorCommentsWidget-rangeBackground: rgba(0, 144, 241, 0.1);
--vscode-editorCommentsWidget-rangeActiveBackground: rgba(0, 144, 241, 0.1);
--vscode-editorGutter-commentRangeForeground: #d5d8e9;
--vscode-editorOverviewRuler-commentForeground: #d5d8e9;
--vscode-editorOverviewRuler-commentUnresolvedForeground: #d5d8e9;
--vscode-editorGutter-commentGlyphForeground: #000000;
--vscode-editorGutter-commentUnresolvedGlyphForeground: #000000;
--vscode-debugToolBar-background: #f3f3f3;
--vscode-debugIcon-startForeground: #388a34;
--vscode-editor-stackFrameHighlightBackground: rgba(255, 255, 102, 0.45);
--vscode-editor-focusedStackFrameHighlightBackground: rgba(206, 231, 206, 0.45);
--vscode-mergeEditor-change-background: rgba(155, 185, 85, 0.2);
--vscode-mergeEditor-change-word-background: rgba(156, 204, 44, 0.4);
--vscode-mergeEditor-changeBase-background: #ffcccc;
--vscode-mergeEditor-changeBase-word-background: #ffa3a3;
--vscode-mergeEditor-conflict-unhandledUnfocused-border: #ffa600;
--vscode-mergeEditor-conflict-unhandledFocused-border: #ffa600;
--vscode-mergeEditor-conflict-handledUnfocused-border: rgba(134, 134, 134, 0.29);
--vscode-mergeEditor-conflict-handledFocused-border: rgba(193, 193, 193, 0.8);
--vscode-mergeEditor-conflict-handled-minimapOverViewRuler: rgba(173, 172, 168, 0.93);
--vscode-mergeEditor-conflict-unhandled-minimapOverViewRuler: #fcba03;
--vscode-mergeEditor-conflictingLines-background: rgba(255, 234, 0, 0.28);
--vscode-mergeEditor-conflict-input1-background: rgba(64, 200, 174, 0.2);
--vscode-mergeEditor-conflict-input2-background: rgba(64, 166, 255, 0.2);
--vscode-settings-headerForeground: #444444;
--vscode-settings-settingsHeaderHoverForeground: rgba(68, 68, 68, 0.7);
--vscode-settings-modifiedItemIndicator: #66afe0;
--vscode-settings-headerBorder: rgba(128, 128, 128, 0.35);
--vscode-settings-sashBorder: rgba(128, 128, 128, 0.35);
--vscode-settings-dropdownBackground: #ffffff;
--vscode-settings-dropdownForeground: #616161;
--vscode-settings-dropdownBorder: #cecece;
--vscode-settings-dropdownListBorder: #c8c8c8;
--vscode-settings-checkboxBackground: #ffffff;
--vscode-settings-checkboxForeground: #616161;
--vscode-settings-checkboxBorder: #919191;
--vscode-settings-textInputBackground: #ffffff;
--vscode-settings-textInputForeground: #616161;
--vscode-settings-textInputBorder: #cecece;
--vscode-settings-numberInputBackground: #ffffff;
--vscode-settings-numberInputForeground: #616161;
--vscode-settings-numberInputBorder: #cecece;
--vscode-settings-focusedRowBackground: rgba(232, 232, 232, 0.6);
--vscode-settings-rowHoverBackground: rgba(232, 232, 232, 0.3);
--vscode-settings-focusedRowBorder: #0090f1;
--vscode-terminal-foreground: #333333;
--vscode-terminal-selectionBackground: #add6ff;
--vscode-terminal-inactiveSelectionBackground: #e5ebf1;
--vscode-terminalCommandDecoration-defaultBackground: rgba(0, 0, 0, 0.25);
--vscode-terminalCommandDecoration-successBackground: #2090d3;
--vscode-terminalCommandDecoration-errorBackground: #e51400;
--vscode-terminalOverviewRuler-cursorForeground: rgba(160, 160, 160, 0.8);
--vscode-terminal-border: rgba(128, 128, 128, 0.35);
--vscode-terminal-findMatchBackground: #a8ac94;
--vscode-terminal-hoverHighlightBackground: rgba(173, 214, 255, 0.07);
--vscode-terminal-findMatchHighlightBackground: rgba(234, 92, 0, 0.33);
--vscode-terminalOverviewRuler-findMatchForeground: rgba(209, 134, 22, 0.49);
--vscode-terminal-dropBackground: rgba(38, 119, 203, 0.18);
--vscode-terminalStickyScrollHover-background: #f0f0f0;
--vscode-testing-iconFailed: #f14c4c;
--vscode-testing-iconErrored: #f14c4c;
--vscode-testing-iconPassed: #73c991;
--vscode-testing-runAction: #73c991;
--vscode-testing-iconQueued: #cca700;
--vscode-testing-iconUnset: #848484;
--vscode-testing-iconSkipped: #848484;
--vscode-testing-peekBorder: #e51400;
--vscode-testing-messagePeekBorder: #1a85ff;
--vscode-testing-peekHeaderBackground: rgba(229, 20, 0, 0.1);
--vscode-testing-messagePeekHeaderBackground: rgba(26, 133, 255, 0.1);
--vscode-testing-coveredBackground: rgba(156, 204, 44, 0.25);
--vscode-testing-coveredBorder: rgba(156, 204, 44, 0.19);
--vscode-testing-coveredGutterBackground: rgba(156, 204, 44, 0.15);
--vscode-testing-uncoveredBranchBackground: #ff9999;
--vscode-testing-uncoveredBackground: rgba(255, 0, 0, 0.2);
--vscode-testing-uncoveredBorder: rgba(255, 0, 0, 0.15);
--vscode-testing-uncoveredGutterBackground: rgba(255, 0, 0, 0.3);
--vscode-testing-coverCountBadgeBackground: #c4c4c4;
--vscode-testing-coverCountBadgeForeground: #333333;
--vscode-testing-message-error-decorationForeground: #e51400;
--vscode-testing-message-error-lineBackground: rgba(255, 0, 0, 0.2);
--vscode-testing-message-info-decorationForeground: rgba(0, 0, 0, 0.5);
--vscode-welcomePage-tileBackground: #f3f3f3;
--vscode-welcomePage-tileHoverBackground: #dbdbdb;
--vscode-welcomePage-tileBorder: rgba(0, 0, 0, 0.1);
--vscode-welcomePage-progress-background: #ffffff;
--vscode-welcomePage-progress-foreground: #006ab1;
--vscode-walkthrough-stepTitle-foreground: #000000;
--vscode-walkThrough-embeddedEditorBackground: #f4f4f4;
--vscode-inlineChat-background: #f3f3f3;
--vscode-inlineChat-border: #c8c8c8;
--vscode-inlineChat-shadow: rgba(0, 0, 0, 0.16);
--vscode-inlineChat-regionHighlight: rgba(173, 214, 255, 0.15);
--vscode-inlineChatInput-border: #c8c8c8;
--vscode-inlineChatInput-focusBorder: #0090f1;
--vscode-inlineChatInput-placeholderForeground: #767676;
--vscode-inlineChatInput-background: #ffffff;
--vscode-inlineChatDiff-inserted: rgba(156, 204, 44, 0.13);
--vscode-editorOverviewRuler-inlineChatInserted: rgba(156, 204, 44, 0.2);
--vscode-inlineChatDiff-removed: rgba(255, 0, 0, 0.1);
--vscode-editorOverviewRuler-inlineChatRemoved: rgba(255, 0, 0, 0.16);
--vscode-debugExceptionWidget-border: #a31515;
--vscode-debugExceptionWidget-background: #f1dfde;
--vscode-statusBar-debuggingBackground: #cc6633;
--vscode-statusBar-debuggingForeground: #ffffff;
--vscode-commandCenter-debuggingBackground: rgba(204, 102, 51, 0.26);
--vscode-editorGutter-modifiedBackground: #2090d3;
--vscode-editorGutter-addedBackground: #48985d;
--vscode-editorGutter-deletedBackground: #e51400;
--vscode-minimapGutter-modifiedBackground: #2090d3;
--vscode-minimapGutter-addedBackground: #48985d;
--vscode-minimapGutter-deletedBackground: #e51400;
--vscode-editorOverviewRuler-modifiedForeground: rgba(32, 144, 211, 0.6);
--vscode-editorOverviewRuler-addedForeground: rgba(72, 152, 93, 0.6);
--vscode-editorOverviewRuler-deletedForeground: rgba(229, 20, 0, 0.6);
--vscode-keybindingTable-headerBackground: rgba(97, 97, 97, 0.04);
--vscode-keybindingTable-rowsBackground: rgba(97, 97, 97, 0.04);
--vscode-debugIcon-breakpointForeground: #e51400;
--vscode-debugIcon-breakpointDisabledForeground: #848484;
--vscode-debugIcon-breakpointUnverifiedForeground: #848484;
--vscode-debugIcon-breakpointCurrentStackframeForeground: #be8700;
--vscode-debugIcon-breakpointStackframeForeground: #89d185;
--vscode-editor-inlineValuesForeground: rgba(0, 0, 0, 0.5);
--vscode-editor-inlineValuesBackground: rgba(255, 200, 0, 0.2);
--vscode-ports-iconRunningProcessForeground: #369432;
--vscode-debugTokenExpression-name: #9b46b0;
--vscode-debugTokenExpression-value: rgba(108, 108, 108, 0.8);
--vscode-debugTokenExpression-string: #a31515;
--vscode-debugTokenExpression-boolean: #0000ff;
--vscode-debugTokenExpression-number: #098658;
--vscode-debugTokenExpression-error: #e51400;
--vscode-debugView-exceptionLabelForeground: #ffffff;
--vscode-debugView-exceptionLabelBackground: #a31515;
--vscode-debugView-stateLabelForeground: #616161;
--vscode-debugView-stateLabelBackground: rgba(136, 136, 136, 0.27);
--vscode-debugView-valueChangedHighlight: #569cd6;
--vscode-debugConsole-infoForeground: #1a85ff;
--vscode-debugConsole-warningForeground: #bf8803;
--vscode-debugConsole-errorForeground: #a1260d;
--vscode-debugConsole-sourceForeground: #616161;
--vscode-debugConsoleInputIcon-foreground: #616161;
--vscode-debugIcon-pauseForeground: #007acc;
--vscode-debugIcon-stopForeground: #a1260d;
--vscode-debugIcon-disconnectForeground: #a1260d;
--vscode-debugIcon-restartForeground: #388a34;
--vscode-debugIcon-stepOverForeground: #007acc;
--vscode-debugIcon-stepIntoForeground: #007acc;
--vscode-debugIcon-stepOutForeground: #007acc;
--vscode-debugIcon-continueForeground: #007acc;
--vscode-debugIcon-stepBackForeground: #007acc;
--vscode-notebook-cellBorderColor: #e8e8e8;
--vscode-notebook-focusedEditorBorder: #0090f1;
--vscode-notebookStatusSuccessIcon-foreground: #388a34;
--vscode-notebookEditorOverviewRuler-runningCellForeground: #388a34;
--vscode-notebookStatusErrorIcon-foreground: #a1260d;
--vscode-notebookStatusRunningIcon-foreground: #616161;
--vscode-notebook-cellToolbarSeparator: rgba(128, 128, 128, 0.35);
--vscode-notebook-selectedCellBackground: rgba(200, 221, 241, 0.31);
--vscode-notebook-selectedCellBorder: #e8e8e8;
--vscode-notebook-focusedCellBorder: #0090f1;
--vscode-notebook-inactiveFocusedCellBorder: #e8e8e8;
--vscode-notebook-cellStatusBarItemHoverBackground: rgba(0, 0, 0, 0.08);
--vscode-notebook-cellInsertionIndicator: #0090f1;
--vscode-notebookScrollbarSlider-background: rgba(100, 100, 100, 0.4);
--vscode-notebookScrollbarSlider-hoverBackground: rgba(100, 100, 100, 0.7);
--vscode-notebookScrollbarSlider-activeBackground: rgba(0, 0, 0, 0.6);
--vscode-notebook-symbolHighlightBackground: rgba(253, 255, 0, 0.2);
--vscode-notebook-cellEditorBackground: #f3f3f3;
--vscode-notebook-editorBackground: #ffffff;
--vscode-interactive-activeCodeBorder: #007acc;
--vscode-interactive-inactiveCodeBorder: #e4e6f1;
--vscode-scm-historyItemAdditionsForeground: #587c0c;
--vscode-scm-historyItemDeletionsForeground: #ad0707;
--vscode-scm-historyItemStatisticsBorder: rgba(97, 97, 97, 0.2);
--vscode-scm-historyItemSelectedStatisticsBorder: rgba(255, 255, 255, 0.2);
--vscode-searchEditor-textInputBorder: #cecece;
--vscode-extensionButton-background: #007acc;
--vscode-extensionButton-foreground: #ffffff;
--vscode-extensionButton-hoverBackground: #0062a3;
--vscode-extensionButton-separator: rgba(255, 255, 255, 0.4);
--vscode-extensionButton-prominentBackground: #007acc;
--vscode-extensionButton-prominentForeground: #ffffff;
--vscode-extensionButton-prominentHoverBackground: #0062a3;
--vscode-extensionIcon-starForeground: #df6100;
--vscode-extensionIcon-verifiedForeground: #006ab1;
--vscode-extensionIcon-preReleaseForeground: #1d9271;
--vscode-extensionIcon-sponsorForeground: #b51e78;
--vscode-terminal-ansiBlack: #000000;
--vscode-terminal-ansiRed: #cd3131;
--vscode-terminal-ansiGreen: #00bc00;
--vscode-terminal-ansiYellow: #949800;
--vscode-terminal-ansiBlue: #0451a5;
--vscode-terminal-ansiMagenta: #bc05bc;
--vscode-terminal-ansiCyan: #0598bc;
--vscode-terminal-ansiWhite: #555555;
--vscode-terminal-ansiBrightBlack: #666666;
--vscode-terminal-ansiBrightRed: #cd3131;
--vscode-terminal-ansiBrightGreen: #14ce14;
--vscode-terminal-ansiBrightYellow: #b5ba00;
--vscode-terminal-ansiBrightBlue: #0451a5;
--vscode-terminal-ansiBrightMagenta: #bc05bc;
--vscode-terminal-ansiBrightCyan: #0598bc;
--vscode-terminal-ansiBrightWhite: #a5a5a5;
--vscode-gitDecoration-addedResourceForeground: #587c0c;
--vscode-gitDecoration-modifiedResourceForeground: #895503;
--vscode-gitDecoration-deletedResourceForeground: #ad0707;
--vscode-gitDecoration-renamedResourceForeground: #007100;
--vscode-gitDecoration-untrackedResourceForeground: #007100;
--vscode-gitDecoration-ignoredResourceForeground: #8e8e90;
--vscode-gitDecoration-stageModifiedResourceForeground: #895503;
--vscode-gitDecoration-stageDeletedResourceForeground: #ad0707;
--vscode-gitDecoration-conflictingResourceForeground: #ad0707;
--vscode-gitDecoration-submoduleResourceForeground: #1258a7;
--vscode-rust_analyzer-syntaxTreeBorder: #b700ff;
`;

function findErrorTargetInBundles(bundles: BodyBundle[]) {
  const traitErrors = _.map(bundles, b =>
    _.map(b.body.traitErrors, e =>
      _.map(
        e.hashes,
        hash =>
          ({
            file: b.filename,
            bodyIdx: b.body.hash,
            exprIdx: e.idx,
            hash,
          } as ErrorJumpTargetInfo)
      )
    )
  );

  const ambiguityErrors = _.map(bundles, b =>
    _.map(
      b.body.ambiguityErrors,
      e =>
        ({
          file: b.filename,
          bodyIdx: b.body.hash,
          exprIdx: e.idx,
          hash: b.body.obligations[b.body.exprs[e.idx].obligations[0]].hash,
        } as ErrorJumpTargetInfo)
    )
  );

  const potentialTargets = _.flatten([
    ..._.flatten(traitErrors),
    ...ambiguityErrors,
  ]);
  return potentialTargets[0];
}

export function webHtml(
  title: string,
  filename: Filename,
  bundles: BodyBundle[],
  evalMode: EvaluationMode = "rank"
) {
  const config: PanoptesConfig = {
    type: "WEB_BUNDLE",
    target: findErrorTargetInBundles(bundles),
    data: [[filename, bundles.map(b => b.body)]],
    closedSystem: bundles,
    evalMode,
  };

  const panoptesDir = path.resolve(__dirname, "..", "..", "panoptes");
  const scriptUri = path.resolve(panoptesDir, "dist", "panoptes.iife.js");
  const styleUri = path.resolve(panoptesDir, "dist", "style.css");
  const codiconsUri = path.resolve(
    panoptesDir,
    "node_modules",
    "@vscode/codicons",
    "dist",
    "codicon.css"
  );

  const url = (uri: string) => `http://localhost:${PORT}${uri}`;
  const styleUrl = url(styleUri);
  const codiconsUrl = url(codiconsUri);
  const scriptUrl = url(scriptUri);
  const configString = configToString(config);

  return `
      <!DOCTYPE html>
      <html lang="en">
      <head style="${defaultStyles}">
          <meta charset="UTF-8">
          <meta name="viewport" content="width=device-width, initial-scale=1.0">
          <title>${title}</title>
          <link rel="stylesheet" type="text/css" href="${styleUrl}">
          <link rel="stylesheet" type="text/css" href="${codiconsUrl}">
      </head>
      <body>
          <div id="root" class=${ConfigConsts.EMBED_NAME} style="width: 100%; height: 100%;" data-config=${configString}></div>
          <script src="${scriptUrl}"></script>
      </body>
      </html>
    `;
}
