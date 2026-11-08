import { ImageResponse } from "next/og";
import type { CSSProperties } from "react";

export const alt = "Snipslate: snip, translate and copy text on screen";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

const blue = "#2563EB";
const white = "#FFFFFF";
const amber = "#FBBF24";
const ink = "#0F172A";
const muted = "#475569";
const paper = "#F1F5F9";

const edge = `8px solid ${white}`;
const base: CSSProperties = { position: "absolute", width: 22, height: 22 };

const topLeft: CSSProperties = {
  ...base,
  top: 44,
  left: 44,
  borderTop: edge,
  borderLeft: edge,
};
const topRight: CSSProperties = {
  ...base,
  top: 44,
  right: 44,
  borderTop: edge,
  borderRight: edge,
};
const bottomLeft: CSSProperties = {
  ...base,
  bottom: 44,
  left: 44,
  borderBottom: edge,
  borderLeft: edge,
};
const bottomRight: CSSProperties = {
  ...base,
  bottom: 44,
  right: 44,
  borderBottom: edge,
  borderRight: edge,
};

const tile: CSSProperties = {
  position: "relative",
  display: "flex",
  flexShrink: 0,
  width: 240,
  height: 240,
  borderRadius: 52,
  background: blue,
};
const line: CSSProperties = {
  position: "absolute",
  top: 89,
  left: 70,
  width: 100,
  height: 24,
  borderRadius: 12,
  background: white,
};
const marker: CSSProperties = {
  position: "absolute",
  top: 132,
  left: 70,
  width: 72,
  height: 24,
  borderRadius: 12,
  background: amber,
};

const page: CSSProperties = {
  display: "flex",
  alignItems: "center",
  width: "100%",
  height: "100%",
  padding: 80,
  background: paper,
  fontFamily: "sans-serif",
};
const copy: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  marginLeft: 72,
};
const name: CSSProperties = { fontSize: 84, fontWeight: 700, color: ink };
const headline: CSSProperties = {
  marginTop: 20,
  maxWidth: 700,
  fontSize: 40,
  color: muted,
};

export default function Image() {
  return new ImageResponse(
    <div style={page}>
      <div style={tile}>
        <div style={topLeft} />
        <div style={topRight} />
        <div style={bottomLeft} />
        <div style={bottomRight} />
        <div style={line} />
        <div style={marker} />
      </div>
      <div style={copy}>
        <div style={name}>Snipslate</div>
        <div style={headline}>
          Snip any text on your screen. Read it in your language.
        </div>
      </div>
    </div>,
    size,
  );
}
