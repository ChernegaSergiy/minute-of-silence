import { useState, useEffect } from "react";
import { MEDIA_CACHE_NAME } from "../utils/mediaCache";

export function useCachedImage(url?: string | null): string | undefined {
  const [cachedSrc, setCachedSrc] = useState<string | undefined>(undefined);

  useEffect(() => {
    if (!url) {
      setCachedSrc(undefined);
      return;
    }

    let isMounted = true;
    let objectUrl: string | null = null;

    const loadAndCache = async () => {
      try {
        const cache = await caches.open(MEDIA_CACHE_NAME);
        let response = await cache.match(url);

        if (!response) {
          // Fallback network fetch if the background prefetch didn't finish yet
          response = await fetch(url);
          if (response.ok) {
            await cache.put(url, response.clone());
          }
        }

        if (response?.ok && isMounted) {
          const blob = await response.blob();
          objectUrl = URL.createObjectURL(blob);
          setCachedSrc(objectUrl);
        }
      } catch (err) {
        console.error("Failed to load cached image:", err);
        // Fallback to original url on failure
        if (isMounted) setCachedSrc(url);
      }
    };

    loadAndCache();

    return () => {
      isMounted = false;
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [url]);

  return cachedSrc;
}
