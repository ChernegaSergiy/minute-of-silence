import { ImgHTMLAttributes, ReactNode, useEffect, useState } from "react";
import { useCachedImage } from "../hooks/useCachedImage";

interface CachedImageProps extends ImgHTMLAttributes<HTMLImageElement> {
  srcUrl?: string | null;
  fallback?: ReactNode;
}

export const CachedImage = ({ srcUrl, fallback = null, onError, ...props }: CachedImageProps) => {
  const cachedSrc = useCachedImage(srcUrl);
  const [errored, setErrored] = useState(false);

  useEffect(() => {
    setErrored(false);
  }, [srcUrl]);

  const src = cachedSrc || srcUrl;

  if (!src || errored) return <>{fallback}</>;

  return (
    <img
      src={src}
      onError={(e) => {
        setErrored(true);
        onError?.(e);
      }}
      {...props}
    />
  );
};
