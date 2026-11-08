import { Callout } from "@/components/callout";
import { DocPager } from "@/components/doc-pager";
import { Inline } from "@/components/inline";
import { DOCS } from "@/lib/docs-nav";
import type { Block } from "@/lib/docs-types";
import { titleClass } from "@/lib/ui";

const cell = "border-b border-line px-4 py-2.5 align-top";
const thHead = "border-b border-line px-4 py-2.5 align-top font-semibold";
const thRow = "border-b border-line px-4 py-2.5 align-top font-medium";

function keyOf(block: Block): string {
  if (block.type === "ul" || block.type === "ol") {
    return `${block.type}:${block.items.join("|")}`;
  }
  if (block.type === "table") {
    return `table:${block.head.join("|")}:${block.rows.length}`;
  }
  return `${block.type}:${block.text}`;
}

type RowProps = { row: string[] };

function Row({ row }: RowProps) {
  const [first, ...rest] = row;
  return (
    <tr>
      <th className={thRow} scope="row">
        <Inline text={first} />
      </th>
      {rest.map((text) => (
        <td className={cell} key={text}>
          <Inline text={text} />
        </td>
      ))}
    </tr>
  );
}

type BlockProps = { block: Block };

function BlockView({ block }: BlockProps) {
  switch (block.type) {
    case "p":
      return (
        <p className="mt-4">
          <Inline text={block.text} />
        </p>
      );
    case "h2":
      return (
        <h2 className="mt-12 text-heading font-semibold text-fg" id={block.id}>
          {block.text}
        </h2>
      );
    case "ul":
      return (
        <ul className="mt-4 list-disc space-y-2 pl-6 marker:text-muted">
          {block.items.map((item) => (
            <li key={item}>
              <Inline text={item} />
            </li>
          ))}
        </ul>
      );
    case "ol":
      return (
        <ol className="mt-4 list-decimal space-y-2 pl-6 marker:text-muted">
          {block.items.map((item) => (
            <li key={item}>
              <Inline text={item} />
            </li>
          ))}
        </ol>
      );
    case "code":
      return (
        <pre className="mt-4 overflow-x-auto rounded-card bg-sunken p-4 font-mono text-small">
          <code>{block.text}</code>
        </pre>
      );
    case "table":
      return (
        <div className="mt-6 overflow-x-auto rounded-card border border-line bg-surface">
          <table className="w-full text-left text-small">
            <thead>
              <tr>
                {block.head.map((name) => (
                  <th className={thHead} key={name} scope="col">
                    {name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row) => (
                <Row key={row.join("|")} row={row} />
              ))}
            </tbody>
          </table>
        </div>
      );
    case "callout":
      return (
        <Callout kind={block.kind}>
          <Inline text={block.text} />
        </Callout>
      );
  }
}

type Props = { href: string; blocks: Block[] };

export function DocPage({ href, blocks }: Props) {
  const index = DOCS.findIndex((doc) => doc.href === href);
  const entry = DOCS[index];
  if (!entry) {
    throw new Error(`Unknown docs page: ${href}`);
  }
  const prev = index > 0 ? DOCS.at(index - 1) : undefined;
  return (
    <article>
      <h1 className={titleClass}>{entry.title}</h1>
      <p className="mt-3 text-lead text-muted">{entry.description}</p>
      {blocks.map((block) => (
        <BlockView block={block} key={keyOf(block)} />
      ))}
      <DocPager next={DOCS.at(index + 1)} prev={prev} />
    </article>
  );
}
