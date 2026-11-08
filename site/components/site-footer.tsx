import { TextLink } from "@/components/text-link";
import { AUTHOR_URL, REPO_URL } from "@/lib/site";
import { container } from "@/lib/ui";

const links = [
  { href: "/docs", label: "Docs" },
  { href: REPO_URL, label: "GitHub" },
  { href: `${REPO_URL}/blob/main/LICENSE`, label: "MIT license" },
  { href: AUTHOR_URL, label: "Made by Akshit Bansal" },
];

export function SiteFooter() {
  return (
    <footer className="border-t border-line">
      <div className={container}>
        <ul className="flex flex-wrap gap-x-6 gap-y-2 py-10 text-small">
          {links.map((link) => (
            <li key={link.href}>
              <TextLink href={link.href}>{link.label}</TextLink>
            </li>
          ))}
        </ul>
      </div>
    </footer>
  );
}
