# Home Tab & Caching

## Overview

The Home tab (`src/components/HomeTab.tsx`) is the main feed screen. It renders story groups, posts with media carousels, and author avatars, and it is the single entry point for feed synchronisation: every feed download, cache update, media prefetch and garbage collection happens inside `fetchFeed()`.

Design goal: **offline-first, instant startup**. The tab must show content from the first frame, and only then check in the background whether anything changed on the server.

## Data flow: `fetchFeed()`

```
fetchFeed(forceRefresh = false)
  ├─ 1. Read local cache (feed_cache.json)          ← instant render
  │       ├─ cache hit  → setFeed(cached), hide skeleton
  │       └─ cache miss → show skeleton (min 600 ms)
  │
  ├─ 2. Network check
  │       ├─ normal start → fetch(FEED_URL, { cache: "no-cache" })
  │       │                  → Conditional GET (If-None-Match)
  │       │                  → 304: body comes from HTTP cache, ~0 bytes
  │       │                  → 200: new JSON, new ETag
  │       └─ F5 / Ctrl+R → fetch(FEED_URL, { cache: "reload" })
  │                          → bypass cache, always full download
  │
  ├─ 3. On new data
  │       ├─ saveFeedCache(data)                     → rewrite feed_cache.json
  │       ├─ prefetchMedia(urls)                     → fill media cache (fire & forget)
  │       ├─ gcMediaCache(urls)                      → drop media no longer in feed
  │       └─ prune viewedStories (localStorage)
  │
  └─ 4. On failure (no network)
          └─ keep whatever is already on screen; error only if there is no cache
```

## Caching layers

There are three independent caches. They solve different problems and do not interfere with each other.

| # | Cache | Owner | Purpose |
|---|-------|-------|---------|
| 1 | `feed_cache.json` | the app (Tauri plugin-store) | offline feed + instant startup |
| 2 | HTTP cache (ETag → 304) | the webview | avoid re-downloading unchanged `feed.json` |
| 3 | `khvylyna-media-v1` | the webview (Cache API) | offline media, no duplicate downloads |

### 1. Local feed cache — `feed_cache.json`

Written by `saveFeedCache()` / read by `getFeedCache()` (`src/utils/api.ts`), backed by the Tauri `LazyStore` plugin.

- Linux: `~/.local/share/ua.pp.khvylyna.MinuteOfSilence/feed_cache.json`
- Contains the full parsed feed, not metadata.

This is what makes the tab render instantly and survive without internet.

### 2. HTTP cache — Conditional GET with ETag

Cloudflare Pages serves `feed.json` with:

```
etag: "936df17df020e12583dd543d210ab772"
cache-control: public, max-age=0, must-revalidate
```

The one-line implementation in `fetchFeed()`:

```ts
const response = await fetch(FEED_URL, { cache: forceRefresh ? "reload" : "no-cache" });
```

- `no-cache` — the webview revalidates before using its cached copy: it sends `If-None-Match` with the stored ETag itself. Unchanged feed → `304 Not Modified` with an empty body; the JSON is then served from the webview's HTTP cache. Changed feed → `200` + new JSON + new ETag.
- `reload` — manual refresh (F5 / Ctrl+R) bypasses the cache completely.

Verified against the live endpoint (Chrome net-log):

```
request 1 → 200, etag: W/"936df17d…"
request 2 → if-none-match: W/"936df17d…"  →  :status: 304   (no body)
OPTIONS   → 0 requests (no CORS preflight)
```

**Why the ETag is not handled manually in app code.** Storing the ETag in a `meta.json` and sending `If-None-Match` from JS is impossible in a webview without server changes:

- a custom request header triggers a CORS preflight, and Cloudflare Pages answers `OPTIONS` with `405`;
- the `ETag` response header is not CORS-safelisted and the endpoint sends no `Access-Control-Expose-Headers`, so `response.headers.get("etag")` returns `null`.

Letting the browser own the ETag gives the same result (0 bytes when nothing changed) with zero server configuration.

### 3. Media cache — `khvylyna-media-v1`

A named Cache API store (`src/utils/mediaCache.ts`). Each entry is a full HTTP response: metadata file + `-blob` file with the image bytes.

Writers:

- `prefetchMedia()` — called after every successful feed download; downloads only URLs that are not cached yet;
- `useCachedImage()` — fallback write when the prefetch has not finished yet.

Reader:

- `useCachedImage()` — `cache.match(url)` → blob → object URL → `<img>`.

Cleanup:

- `gcMediaCache()` — deletes every entry whose URL is no longer mentioned in the feed (runs after each successful download).

Media filenames are immutable by design: the CMS generates `crypto.randomUUID()` names on upload (`moment-of-honor-cms/src/routes/media.ts`) and never overwrites them, so cache busting through filenames already works and no conditional requests are needed for images.

## Component tree

All media rendering goes through two wrappers; only they call the cache hook:

```
HomeTab
├── avatars (399, 415) ────────────→ CachedAvatar           ┐
└── post media (420) ──→ PostMediaCarousel ──→ CachedImage  ┤
                                                            ├──→ useCachedImage ──→ media cache khvylyna-media-v1
StoryViewer                                                 │
├── avatar (297) ──────────────────→ CachedAvatar           ┤
└── story media (319) ─────────────→ CachedImage            ┘
```

- `CachedImage` — `<img>` wrapper: accepts `srcUrl?: string | null`, renders a `fallback` node when there is no URL or the image failed to load, resets its error state when `srcUrl` changes.
- `CachedAvatar` — Fluent UI `Avatar` wrapper with the same cache hook.
- `useCachedImage` (`src/hooks/useCachedImage.ts`) — the only code that talks to `khvylyna-media-v1`.

## Offline behaviour

1. Startup: feed renders from `feed_cache.json` before any network activity.
2. The background `fetch` fails without connectivity → the `catch` block keeps the already rendered feed; the error screen appears only when there is no cache at all.
3. Media renders from `khvylyna-media-v1`; already cached images are shown without network.
4. The HTTP cache (layer 2) does not participate in offline mode — it is a bandwidth optimisation, not an offline store.

## Related state

- `viewedStories` (localStorage) — ids of seen stories, pruned to the ids present in the current feed.
- Skeleton is shown for at least 600 ms to avoid flashing on fast responses.
