import Image from "next/image";
import { frame } from "@/lib/ui";

type Props = {
  src: string;
  alt: string;
  width: number;
  height: number;
  priority?: boolean;
  sizes?: string;
  imageClass?: string;
};

export function Screenshot({
  src,
  alt,
  width,
  height,
  priority,
  sizes = "100vw",
  imageClass = "h-auto w-full",
}: Props) {
  return (
    <div className={frame}>
      <Image
        alt={alt}
        className={imageClass}
        height={height}
        priority={priority}
        sizes={sizes}
        src={src}
        width={width}
      />
    </div>
  );
}
