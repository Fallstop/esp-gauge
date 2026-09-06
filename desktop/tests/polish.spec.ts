import { test, expect, type Page } from '@playwright/test';

async function setup(page: Page) {
  await page.addInitScript(() => {
    const w = window as any;
    const callbacks = new Map();
    const listeners = new Map();
    let next = 1;
    const channel = {
      enabled: true,
      name: '',
      source: 'cpu',
      min_duty: 0,
      max_duty: 100,
      response_ms: 500,
      scale: 100,
      input_min: 0,
      reverse: false,
    };
    const makeSource = (
      id: string,
      name: string,
      group = 'This computer',
      options: any[] = [],
      unit = '%',
      scale = 100,
    ) => ({
      id,
      name,
      group,
      options,
      unit,
      scale,
      minimum: 0,
      description: `Reading ${name.toLowerCase()}.`,
    });
    w.fixture = {
      connected: true,
      device: 'fixture',
      path: '/dev/fixture',
      firmware: '2.3.0',
      paused: false,
      candidates: ['/dev/fixture'],
      devices: [{ id: 'fixture', path: '/dev/fixture' }],
      error: null,
      config: { version: 2, channels: Array.from({ length: 6 }, () => ({ ...channel })) },
      metrics: { cpu: 35, memory: 48, network_down: 0.23, volume: 46 },
      board: { positions: [0.35, 0.48, 0, 0, 0, 0], available: [true, true, false, false, false, false] },
      sources: [
        makeSource('disk', 'Disk space', 'This computer', [
          { id: '/', name: 'Macintosh HD · /' },
          { id: '/Volumes/Work', name: 'Work drive' },
        ]),
        makeSource('gpu', 'GPU usage'),
        makeSource('cpu_temperature', 'CPU temperature'),
        makeSource('gpu_temperature', 'GPU temperature'),
        makeSource('volume', 'System volume'),
        makeSource('audio', 'Audio level'),
        makeSource('codex_working', 'Working agents', 'Codex'),
        makeSource('supertracker_product', 'Product price', 'Super Tracker', [], 'NZ$', 20),
      ],
    };
    w.updateFixture = {
      checked: true,
      app_version: '',
      firmware_version: '2.3.0',
      busy: false,
      stage: '',
      progress: 0,
      operation: '',
      indeterminate: false,
      error: null,
    };
    w.emitFixture = (event: string, payload: any) => {
      for (const id of listeners.get(event) ?? []) callbacks.get(id)?.({ event, id: 1, payload });
    };
    w.isTauri = true;
    w.__TAURI_INTERNALS__ = {
      transformCallback: (fn: any) => {
        const id = next++;
        callbacks.set(id, fn);
        return id;
      },
      unregisterCallback: (id: number) => callbacks.delete(id),
      invoke: async (cmd: string, args: any) => {
        if (cmd === 'snapshot') return w.fixture;
        if (cmd === 'update_status') return w.updateFixture;
        if (cmd === 'plugin:event|listen') {
          listeners.set(args.event, [...(listeners.get(args.event) ?? []), args.handler]);
          return next++;
        }
        if (cmd === 'plugin:app|version') return '2.3.0';
        if (cmd === 'search_products') return [{ id: 285, name: 'Anchor Blue Milk 2L' }];
        if (cmd === 'product_stores')
          return [
            { id: 42, name: 'New World Tokoroa' },
            { id: 67, name: 'Pak’nSave Rotorua' },
          ];
        if (cmd === 'command' && args.command.op === 'config') {
          w.fixture.config = args.command.config;
          w.emitFixture('state', w.fixture);
        }
        return false;
      },
    };
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Source', exact: true })).toBeVisible();
}

test('category navigation, keyboard search, battery visibility and per-gauge drive selection', async ({
  page,
}) => {
  await setup(page);
  await page.getByRole('button', { name: 'Source', exact: true }).click();
  await expect(page.getByRole('button', { name: 'This computer Processor' })).toBeVisible();
  await expect(page.getByRole('dialog').getByRole('option')).toHaveCount(0);
  await page.getByLabel('Find a source').press('ArrowDown');
  await page.keyboard.press('Enter');
  await expect(page.getByLabel('Find a source')).toBeFocused();
  await expect(page.getByRole('option', { name: 'Battery', exact: true })).toHaveCount(0);
  await page.getByRole('option', { name: 'Disk space' }).click();
  await page.getByLabel('Drive', { exact: true }).selectOption('/Volumes/Work');
  await expect
    .poll(() => page.evaluate(() => (window as any).fixture.config.channels[0].device))
    .toBe('/Volumes/Work');
  expect(await page.evaluate(() => (window as any).fixture.config.channels[1].device)).toBeUndefined();
  await page.getByLabel('Output scale').selectOption('2');
  await expect.poll(() => page.evaluate(() => (window as any).fixture.config.channels[0].curve)).toBe(2);
  await page.getByRole('button', { name: 'Source', exact: true }).click();
  await page.getByLabel('Find a source').fill('volume');
  await page.getByLabel('Find a source').press('Enter');
  await expect(page.getByRole('button', { name: 'Source', exact: true })).toContainText('System volume');
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('product search and store lock persist together', async ({ page }) => {
  await setup(page);
  await page.getByRole('button', { name: 'Source', exact: true }).click();
  await page.getByRole('button', { name: 'Prices Products' }).click();
  await page.getByRole('option', { name: 'Product price' }).click();
  await page.getByLabel('Product', { exact: true }).fill('milk');
  await page.getByRole('button', { name: 'Find', exact: true }).click();
  await page.getByRole('button', { name: 'Anchor Blue Milk 2L' }).click();
  await page.getByLabel('Store', { exact: true }).selectOption('42');
  await expect.poll(() => page.evaluate(() => (window as any).fixture.config.channels[0].store_id)).toBe(42);
  expect(await page.evaluate(() => (window as any).fixture.config.channels[0].product_id)).toBe(285);
  await expect(page.getByText('Locked to this store.', { exact: false })).toBeVisible();
});

test('update progress remains below the board across disconnect, success and failure', async ({ page }) => {
  await setup(page);
  await page.setViewportSize({ width: 880, height: 580 });
  await page.evaluate(() => {
    const w = window as any;
    w.emitFixture('updates', {
      ...w.updateFixture,
      busy: true,
      operation: 'firmware',
      stage: 'Writing firmware',
      progress: 43,
    });
    w.fixture.connected = false;
    w.emitFixture('state', w.fixture);
  });
  await expect(page.getByRole('progressbar')).toHaveAttribute('value', '43');
  await expect(page.getByText('43%', { exact: true })).toBeVisible();
  await expect(page.getByText('Connect board', { exact: true })).toHaveCount(0);
  const progress = await page.locator('.board-update').boundingBox();
  const board = await page.locator('.board-scene').boundingBox();
  expect(progress!.y).toBeGreaterThanOrEqual(board!.y + board!.height);
  expect(progress!.y + progress!.height).toBeLessThan(580);
  await page.screenshot({ path: '/tmp/esp-update-small.png' });
  await page.evaluate(() => {
    const w = window as any;
    w.emitFixture('updates', {
      ...w.updateFixture,
      busy: true,
      operation: 'firmware',
      stage: 'Restarting board',
      indeterminate: true,
    });
  });
  await expect(page.getByRole('progressbar')).not.toHaveAttribute('value');
  await expect(page.getByText(/elapsed/)).toBeVisible();
  await page.evaluate(() => {
    const w = window as any;
    w.emitFixture('updates', {
      ...w.updateFixture,
      operation: 'firmware',
      stage: 'Firmware installed · board verified',
      progress: 100,
    });
  });
  await expect(page.getByText('Firmware installed · board verified')).toBeVisible();
  await page.evaluate(() => {
    const w = window as any;
    w.emitFixture('updates', {
      ...w.updateFixture,
      operation: 'firmware',
      error: 'Reconnect USB and retry.',
    });
  });
  await expect(page.getByRole('alert')).toContainText('Reconnect USB');
  await expect(page.locator('.board-update')).toBeInViewport({ ratio: 1 });
});

test('picker stays within a small window and selected header draws attention', async ({ page }) => {
  await setup(page);
  await page.setViewportSize({ width: 880, height: 580 });
  await page.getByRole('button', { name: 'Source', exact: true }).click();
  const box = await page.getByRole('dialog').boundingBox();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.y).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(880);
  expect(box!.y + box!.height).toBeLessThanOrEqual(580);
  await page.screenshot({ path: '/tmp/esp-picker-small.png' });
  await page.getByLabel('Find a source').press('Escape');
  await expect(page.getByRole('button', { name: 'Source', exact: true })).toBeFocused();
  await expect(page.locator('.port-heading')).toHaveCSS('font-size', '30px');
  await page.screenshot({ path: '/tmp/esp-main-small.png' });
});

test('curved scales require new firmware without creating unsavable edits', async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const w = window as any;
    w.fixture.firmware = '2.2.4';
    w.emitFixture('state', w.fixture);
  });
  await expect(page.getByLabel('Output scale')).toBeDisabled();
  await expect(page.getByLabel('Output scale')).toHaveValue('0');
  await expect(page.getByText('Update board firmware to 2.3 or later to use curved scales.')).toBeVisible();
});
