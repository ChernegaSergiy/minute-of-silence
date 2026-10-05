export const MEDIA_CACHE_NAME = 'khvylyna-media-v1';

export async function prefetchMedia(urls: string[]) {
  try {
    const cache = await caches.open(MEDIA_CACHE_NAME);
    const uncachedUrls: string[] = [];
    
    for (const url of urls) {
      if (!url) continue;
      const match = await cache.match(url);
      if (!match) uncachedUrls.push(url);
    }

    // Only download missing files to avoid redundant network requests
    await Promise.allSettled(
      uncachedUrls.map(async (url) => {
        try {
          const res = await fetch(url);
          if (res.ok) await cache.put(url, res);
        } catch (err) {
          console.warn(`[Prefetch] Failed to fetch ${url}`, err);
        }
      })
    );
  } catch (e) {
    console.error("Failed to open media cache for prefetch", e);
  }
}

export async function gcMediaCache(activeUrls: string[]) {
  try {
    const cache = await caches.open(MEDIA_CACHE_NAME);
    const keys = await cache.keys();
    const activeSet = new Set(activeUrls);
    
    for (const request of keys) {
      if (!activeSet.has(request.url)) {
        await cache.delete(request);
      }
    }
  } catch (e) {
    console.error("Failed to GC media cache", e);
  }
}
