import { kbdClass } from "@/lib/ui";

type Props = { combo: string };

export function Kbd({ combo }: Props) {
  return (
    <span className="inline-flex items-center gap-1 whitespace-nowrap">
      {combo.split("+").map((key) => (
        <kbd className={kbdClass} key={key}>
          {key}
        </kbd>
      ))}
    </span>
  );
}
