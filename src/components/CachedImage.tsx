import { ImgHTMLAttributes } from "react";
import { useCachedImage } from "../hooks/useCachedImage";

interface CachedImageProps extends ImgHTMLAttributes<HTMLImageElement> {
  srcUrl: string;
}

export const CachedImage = ({ srcUrl, ...props }: CachedImageProps) => {
  const cachedSrc = useCachedImage(srcUrl);
  return <img src={cachedSrc || srcUrl} {...props} />;
};
