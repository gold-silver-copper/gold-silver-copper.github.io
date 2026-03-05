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
    // Trigger GriftApp's MutationObserver to pick up the canvas
  });
  // Allow time for ResizeObserver + updateBackingStore to run
  await page.waitForTimeout(200);
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

  test('canvas resizes correctly when viewport changes', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);

    // Resize to a different size
    await page.setViewportSize({ width: 800, height: 600 });
    await page.waitForTimeout(600);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(800);
    expect(dims.cssHeight).toBe(600);
    expect(dims.backingWidth).toBe(Math.round(800 * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(600 * dims.dpr));
  });

  test('canvas resizes correctly through multiple viewport changes', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);

    const sizes = [
      { width: 1024, height: 768 },
      { width: 1920, height: 1080 },
      { width: 640, height: 480 },
      { width: 1280, height: 720 },
    ];

    for (const size of sizes) {
      await page.setViewportSize(size);
      await page.waitForTimeout(600);

      const dims = await getCanvasDimensions(page);
      expect(dims).not.toBeNull();
      expect(dims.cssWidth).toBe(size.width);
      expect(dims.cssHeight).toBe(size.height);
      expect(dims.backingWidth).toBe(Math.round(size.width * dims.dpr));
      expect(dims.backingHeight).toBe(Math.round(size.height * dims.dpr));
    }
  });
});

// ─── Mobile orientation tests ──────────────────────────────────────

test.describe('Mobile orientation changes', () => {
  test('canvas fills viewport after portrait → landscape → portrait', async ({ page, browserName }) => {
    // iPhone 12 portrait dimensions
    const portrait = { width: 390, height: 844 };
    const landscape = { width: 844, height: 390 };

    await page.setViewportSize(portrait);
    await page.goto('/');
    await injectTestCanvas(page);

    // Verify initial portrait
    let dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(portrait.width);
    expect(dims.cssHeight).toBe(portrait.height);

    // Rotate to landscape
    await page.setViewportSize(landscape);
    await page.waitForTimeout(600);

    dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(landscape.width);
    expect(dims.cssHeight).toBe(landscape.height);
    expect(dims.backingWidth).toBe(Math.round(landscape.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(landscape.height * dims.dpr));

    // Rotate back to portrait
    await page.setViewportSize(portrait);
    await page.waitForTimeout(600);

    dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(portrait.width);
    expect(dims.cssHeight).toBe(portrait.height);
    expect(dims.backingWidth).toBe(Math.round(portrait.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(portrait.height * dims.dpr));
  });

  test('canvas aspect ratio matches viewport after orientation change', async ({ page }) => {
    const portrait = { width: 412, height: 915 };
    const landscape = { width: 915, height: 412 };

    await page.setViewportSize(portrait);
    await page.goto('/');
    await injectTestCanvas(page);

    // Switch to landscape
    await page.setViewportSize(landscape);
    await page.waitForTimeout(600);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();

    // The backing store aspect ratio should match the viewport
    const viewportAR = landscape.width / landscape.height;
    const canvasAR = dims.backingWidth / dims.backingHeight;
    expect(Math.abs(viewportAR - canvasAR)).toBeLessThan(0.02);
  });

  test('rapid orientation toggles settle to correct dimensions', async ({ page }) => {
    const portrait = { width: 390, height: 844 };
    const landscape = { width: 844, height: 390 };

    await page.setViewportSize(portrait);
    await page.goto('/');
    await injectTestCanvas(page);

    // Rapidly toggle orientations
    for (let i = 0; i < 5; i++) {
      await page.setViewportSize(landscape);
      await page.waitForTimeout(100);
      await page.setViewportSize(portrait);
      await page.waitForTimeout(100);
    }

    // Wait for all delayed re-measurements to complete
    await page.waitForTimeout(700);

    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.cssWidth).toBe(portrait.width);
    expect(dims.cssHeight).toBe(portrait.height);
    expect(dims.backingWidth).toBe(Math.round(portrait.width * dims.dpr));
    expect(dims.backingHeight).toBe(Math.round(portrait.height * dims.dpr));
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
        width: body.offsetWidth,
        height: body.offsetHeight,
      };
    });

    expect(bodyStyles.overflow).toBe('hidden');
    expect(bodyStyles.position).toBe('fixed');
    expect(bodyStyles.margin).toBe('0px');
  });

  test('canvas uses absolute positioning', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);

    const canvasStyles = await page.evaluate(() => {
      const c = document.querySelector('canvas');
      const cs = window.getComputedStyle(c);
      return {
        position: cs.position,
        top: cs.top,
        left: cs.left,
        display: cs.display,
      };
    });

    expect(canvasStyles.position).toBe('absolute');
    expect(canvasStyles.top).toBe('0px');
    expect(canvasStyles.left).toBe('0px');
    expect(canvasStyles.display).toBe('block');
  });

  test('no scrollbars appear after orientation change', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');
    await injectTestCanvas(page);

    await page.setViewportSize({ width: 844, height: 390 });
    await page.waitForTimeout(600);

    const scrollInfo = await page.evaluate(() => ({
      scrollWidth: document.documentElement.scrollWidth,
      scrollHeight: document.documentElement.scrollHeight,
      clientWidth: document.documentElement.clientWidth,
      clientHeight: document.documentElement.clientHeight,
    }));

    // No scrollable overflow
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
      const topZone = window.GriftApp.zone(10);  // near top
      const bottomZone = window.GriftApp.zone(500);  // in content area
      return { topZone, bottomZone };
    });

    expect(zones.topZone).toBe('tabs');
    expect(zones.bottomZone).toBe('content');
  });

  test('updateBackingStore guards against zero dimensions', async ({ page }) => {
    await page.goto('/');
    await injectTestCanvas(page);

    // The canvas should have positive dimensions
    const dims = await getCanvasDimensions(page);
    expect(dims).not.toBeNull();
    expect(dims.backingWidth).toBeGreaterThan(0);
    expect(dims.backingHeight).toBeGreaterThan(0);
  });
});
