import { Fragment, type ReactNode } from "react";
import { Kbd } from "@/components/kbd";
import { TextLink } from "@/components/text-link";
import { inlineCode } from "@/lib/ui";

// Inline markup used by the docs content: `code`, **bold**, [text](href)
// and {{Ctrl+Alt+T}} for keyboard keys.
const TOKENS = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\)|\{\{[^}]+\}\})/;
const LINK = /^\[([^\]]+)\]\(([^)]+)\)$/;

function renderPart(part: string): ReactNode {
  if (part.startsWith("`")) {
    return <code className={inlineCode}>{part.slice(1, -1)}</code>;
  }
  if (part.startsWith("**")) {
    return <strong className="font-semibold">{part.slice(2, -2)}</strong>;
  }
  if (part.startsWith("{{")) {
    return <Kbd combo={part.slice(2, -2)} />;
  }
  const link = LINK.exec(part);
  if (link) {
    return <TextLink href={link[2]}>{link[1]}</TextLink>;
  }
  return part;
}

type Props = { text: string };

export function Inline({ text }: Props): ReactNode {
  let offset = 0;
  const parts = text.split(TOKENS).filter((part) => part !== "");
  return parts.map((part) => {
    const key = offset;
    offset += part.length;
    return <Fragment key={key}>{renderPart(part)}</Fragment>;
  });
}
