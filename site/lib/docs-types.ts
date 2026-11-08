export type Block =
  | { type: "p"; text: string }
  | { type: "h2"; id: string; text: string }
  | { type: "ul"; items: string[] }
  | { type: "ol"; items: string[] }
  | { type: "code"; text: string }
  | { type: "table"; head: string[]; rows: string[][] }
  | { type: "callout"; kind: "note" | "warning"; text: string };
