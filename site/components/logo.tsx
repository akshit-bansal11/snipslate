type Props = { size?: number };

export function Logo({ size = 28 }: Props) {
  return (
    <svg aria-hidden="true" height={size} viewBox="0 0 100 100" width={size}>
      <title>Snipslate</title>
      <rect
        className="fill-accent"
        height="94"
        rx="20"
        width="94"
        x="3"
        y="3"
      />
      <path
        className="fill-on-accent"
        d="M18 18H40V26H26V40H18ZM60 18H82V40H74V26H60ZM18 60H26V74H40V82H18ZM74 60H82V82H60V74H74ZM34 37H66a5 5 0 0 1 0 10H34a5 5 0 0 1 0-10Z"
      />
      <path
        className="fill-marker"
        d="M34 55H54a5 5 0 0 1 0 10H34a5 5 0 0 1 0-10Z"
      />
    </svg>
  );
}
