// @ts-check
const { test, expect } = require('@playwright/test');

// The WASM app takes time to load; these tests verify the HTML/CSS/JS
// sizing layer works correctly regardless of whether WASM has loaded.
// We inject a test canvas to simulate the ratzilla canvas element.

/** Inject a canvas element mimicking what the WASM framework creates. */
async function injectTestCanvas(page) {
  await page.evaluate(() => {
    if (document.querySelector('canvas')) return;
    const c = document.createElement('canvas');
    document.body.appendChild(c);
  });
  // Wait for the 1-second polling interval to pick up the canvas
  await page.waitForTimeout(1200);
}

/** Read the canvas backing store and CSS display dimensions. */
async function getCanvasDimensions(page) {
  return page.evaluate(() => {
    const c = document.querySelector('canvas');
    if (!c) return null;
    const rect = c.getBoundingClientRect();
    return {
      backingWidth: c.width,
      backingHeight: c.height,
      cssWidth: Math.round(rect.width),
      cssHeight: Math.round(rect.height),
      dpr: window.devicePixelRatio || 1,
    };
  });
}

// ─── Desktop tests ─────────────────────────────────────────────────

test.describe('Desktop viewport sizing', () => {
  test('canvas fills viewport on initial load', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);
    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();

    const viewport = page.viewportSize();
    expect(dims.cssWidth).toBe(viewport.width);
    expect(dims.cssHeight).toBe(viewport.height);
    expect(dims.backingWidth).toBe(Math.round(viewport.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(viewport.height * dims.dpr));
  });

  test('canvas resizes after viewport change', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);

    await page.setViewportSize({ width: 800, height: 600 });
    // Wait for the 1-second poll to catch the change
    await page.waitForTimeout(1200);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(800);
    expect(dims.cssHeight).toBe(600);
    expect(dims.backingWidth).toBe(Math.round(800 * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(600 * dims.dpr));
  });
});

// ─── Mobile orientation tests ──────────────────────────────────────

test.describe('Mobile orientation changes', () => {
  test('portrait to landscape fills viewport', async ({ page }) => {
    const portrait = { width: 390, height: 844 };
    const landscape = { width: 844, height: 390 };

    await page.setViewportSize(portrait);
    await page.goto('/');
    await injectTestCanvas(page);

    // Rotate to landscape
    await page.setViewportSize(landscape);
    await page.waitForTimeout(1200);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(landscape.width);
    expect(dims.cssHeight).toBe(landscape.height);
    expect(dims.backingWidth).toBe(Math.round(landscape.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(landscape.height * dims.dpr));
  });

  test('landscape to portrait fills viewport', async ({ page }) => {
    const landscape = { width: 844, height: 390 };
    const portrait = { width: 390, height: 844 };

    await page.setViewportSize(landscape);
    await page.goto('/');
    await injectTestCanvas(page);

    // Rotate to portrait
    await page.setViewportSize(portrait);
    await page.waitForTimeout(1200);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(portrait.width);
    expect(dims.cssHeight).toBe(portrait.height);
    expect(dims.backingWidth).toBe(Math.round(portrait.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(portrait.height * dims.dpr));
  });

  test('page loaded in landscape sizes correctly', async ({ page }) => {
    const landscape = { width: 851, height: 393 };

    await page.setViewportSize(landscape);
    await page.goto('/');
    await injectTestCanvas(page);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(landscape.width);
    expect(dims.cssHeight).toBe(landscape.height);
    expect(dims.backingWidth).toBe(Math.round(landscape.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(landscape.height * dims.dpr));
  });
});

// ─── CSS layout tests ──────────────────────────────────────────────

test.describe('CSS layout integrity', () => {
  test('body fills viewport with no overflow', async ({ page }) => {
    await page.goto('/');

    const bodyStyles = await page.evaluate(() => {
      const body = document.body;
      const cs = window.getComputedStyle(body);
      return {
        overflow: cs.overflow,
        position: cs.position,
        margin: cs.margin,
      };
    });

    expect(bodyStyles.overflow).toBe('hidden');
    expect(bodyStyles.position).toBe('fixed');
    expect(bodyStyles.margin).toBe('0px');
  });

  test('no scrollbars appear after orientation change', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');
    await injectTestCanvas(page);

    await page.setViewportSize({ width: 844, height: 390 });
    await page.waitForTimeout(1200);

    const scrollInfo = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      scrollHeight: document.documentElement.scrollHeight,
      clientWidth: document.documentElement.clientWidth,
      clientHeight: document.documentElement.clientHeight,
    }));

    expect(scrollInfo.scrollWidth).toBeLessThanOrEqual(scrollInfo.clientWidth + 1);
    expect(scrollInfo.scrollHeight).toBeLessThanOrEqual(scrollInfo.clientHeight + 1);
  });
});

// ─── GriftApp JS interaction layer tests ───────────────────────────

test.describe('GriftApp interaction layer', () => {
  test('GriftApp is defined and exposes expected API', async ({ page }) => {
    await page.goto('/');

    const api = await page.evaluate(() => ({
      hasDispatch: typeof window.GriftApp?.dispatch === 'function',
      hasCanvas: typeof window.GriftApp?.canvas === 'function',
      hasZone: typeof window.GriftApp?.zone === 'function',
      hasGesture: typeof window.GriftApp?.gesture === 'object',
      hasCFG: typeof window.GriftApp?.CFG === 'object',
    }));

    expect(api.hasDispatch).toBe(true);
    expect(api.hasCanvas).toBe(true);
    expect(api.hasZone).toBe(true);
    expect(api.hasGesture).toBe(true);
    expect(api.hasCFG).toBe(true);
  });

  test('zone detection works correctly', async ({ page }) => {
    await page.goto('/');

    const zones = await page.evaluate(() => {
      var h = window.innerHeight || 600;
      var tabFraction = window.GriftApp.CFG.TAB_ZONE_FRACTION;
      var inTabZone = Math.round(h * tabFraction * 0.5);
      var inContentZone = Math.round(h * (tabFraction + 0.5));
      return {
        topZone: window.GriftApp.zone(inTabZone),
        bottomZone: window.GriftApp.zone(inContentZone),
      };
    });

    expect(zones.topZone).toBe('tabs');
    expect(zones.bottomZone).toBe('content');
  });

  test('canvas has positive backing store dimensions', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.backingWidth).toBeGreaterThan(0);
    expect(dims.backingHeight).toBeGreaterThan(0);
  });
});
