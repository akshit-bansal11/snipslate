import { Info, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";

type Props = { kind: "note" | "warning"; children: ReactNode };

export function Callout({ kind, children }: Props) {
  const warning = kind === "warning";
  const Icon = warning ? TriangleAlert : Info;
  const label = warning ? "Warning" : "Note";
  return (
    <div className="mt-6 flex gap-3 rounded-card border border-line-strong border-l-4 border-l-accent bg-surface p-4">
      <Icon
        aria-hidden="true"
        className="mt-0.5 size-5 shrink-0 text-accent-text"
      />
      <div className="min-w-0">
        <p className="font-semibold text-fg">{label}</p>
        <div className="mt-1 text-muted">{children}</div>
      </div>
    </div>
  );
}
