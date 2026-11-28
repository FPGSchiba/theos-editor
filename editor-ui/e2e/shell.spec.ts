// The shell (P11 §5.6, §5.8, §5.9): title bar, toolbar, rail, dock and status bar, the work area
// without a project, and the layouts of the parts whose phase is not built yet.

import { expect, test, type Page } from '@playwright/test';

import { loadRecordings, replay, type Os } from './support/replay';

/** The rail of the fixture project: tower-of-theos's enabled parts, then Code and Settings. */
const RAIL = ['Codex', 'Graph', 'Scripts', 'Concepts', 'Maps', 'Review', 'Changes', 'Code', 'Settings'];

/** The rail entries of the recorded project whose phase is not built yet. */
const UNBUILT = (() => {
  const open = loadRecordings().find((r) => r.name === 'project_open');
  const view = open?.value as { rail: { label: string; phase: string | null }[] } | undefined;
  return (view?.rail ?? []).filter((entry) => entry.phase !== null);
})();

/** Opens the app with the recorded last project. */
async function openProject(page: Page, use: string[] = [], os: Os = 'windows'): Promise<void> {
  await replay(page, { os, use: ['project_last', ...use] });
  await page.goto('/');
  await expect(page.locator('.tu-titlebar')).toContainText('synthetic-codex');
}

// AC: CE-32.1
test('opens_a_project_and_shows_the_rail', async ({ page }) => {
  await openProject(page);
  const rail = page.getByRole('navigation', { name: 'Layout presets' });
  await expect(rail.locator('.tu-rail__label')).toHaveText(RAIL);
  await expect(rail.getByRole('button', { name: 'Codex', exact: true })).toHaveAttribute('aria-current', 'true');
  await expect(rail.locator('[aria-current="true"]')).toHaveCount(1);
  await expect(page.locator('.tu-titlebar')).toContainText('layout · Codex');
});

/**
 * The layout of each unbuilt entry of the fixture rail: the tab titles of its windows in order
 * (reference screens of spec §5.9), and a part of one window's empty-state text where it is pinned.
 */
const LAYOUTS: Record<string, { tabs: string[][]; text?: string }> = {
  Graph: {
    tabs: [['Show'], ['Graph', 'Ledger'], ['Focus', 'Paths']],
    text: 'Open a record and choose Open in graph to start from it.',
  },
  Scripts: {
    tabs: [['Scripts', 'Chronicles'], ['Script'], ['Beat'], ['Reveals'], ['Validation']],
    text: 'Pick a script to write its beats.',
  },
  Concepts: { tabs: [['Concepts'], ['Concept'], ['Version', 'Decisions']] },
  Maps: {
    tabs: [
      ['Maps', 'Recipes', 'Stages'],
      ['Runs', 'Recipe', 'Stage catalogue'],
      ['Seed', 'Engine package'],
    ],
  },
  Review: {
    tabs: [['Review'], ['Preview', 'Diff', 'Context the agent read'], ['Agent', 'MCP tools']],
    text: 'Nothing changes the world until you approve it.',
  },
  Changes: {
    tabs: [['Source control'], ['Diff'], ['History', 'Who wrote what'], ['Branches']],
  },
  Code: { tabs: [['Files'], ['Code']] },
  Settings: {
    tabs: [['Settings'], ['Page'], ['Project file', 'User file'], ['How settings stack']],
    text: 'the project file is shared through git',
  },
};

// AC: CE-32.1
test('unbuilt_entries_show_their_layout', async ({ page }) => {
  expect(UNBUILT.map((entry) => entry.label)).toEqual(Object.keys(LAYOUTS));
  await openProject(page);
  const rail = page.getByRole('navigation', { name: 'Layout presets' });
  for (const entry of UNBUILT) {
    const layout = LAYOUTS[entry.label];
    expect(layout, `${entry.label} has a layout in the table`).toBeDefined();
    await rail.getByRole('button', { name: entry.label, exact: true }).click();
    await expect(rail.getByRole('button', { name: entry.label, exact: true })).toHaveAttribute(
      'aria-current',
      'true',
    );
    await expect(page.locator('.tu-titlebar')).toContainText(`layout · ${entry.label}`);
    const windows = page.locator('.tu-shell__work .tu-window');
    await expect(windows).toHaveCount(layout?.tabs.length ?? 0);
    for (const [index, tabs] of (layout?.tabs ?? []).entries()) {
      const window = windows.nth(index);
      await expect(window.locator('.tu-tab, .tu-tabbar__title')).toHaveText(tabs);
      await expect(window).toContainText(`Arrives with ${entry.phase ?? ''}.`);
    }
    if (layout?.text !== undefined) {
      await expect(page.locator('.tu-shell__work')).toContainText(layout.text);
    }
  }
});

// AC: CE-32.1
test('no_project_offers_open', async ({ page }) => {
  await replay(page, { os: 'windows', use: ['project_last-none'] });
  await page.goto('/');
  const work = page.locator('.tu-shell__work');
  await expect(work).toContainText('Open a content project: Ctrl+O');
  await expect(page.getByRole('navigation', { name: 'Layout presets' })).toBeHidden();
});

test('a_failing_project_last_shows_its_error', async ({ page }) => {
  const fail = { project_last: { kind: 'io', message: 'the state file cannot be read' } };
  await replay(page, { os: 'windows', fail });
  await page.goto('/');
  const work = page.locator('.tu-shell__work');
  await expect(work).toContainText('Open a content project: Ctrl+O');
  await expect(work.getByRole('alert')).toHaveText(
    'The last project could not be read: the state file cannot be read',
  );
});

test('a_failing_app_info_keeps_the_defaults_and_says_so', async ({ page }) => {
  const fail = { app_info: { kind: 'io', message: 'no answer' } };
  await replay(page, { os: 'macos', use: ['project_last-none'], fail });
  await page.goto('/');
  const work = page.locator('.tu-shell__work');
  await expect(work).toContainText('Open a content project: Ctrl+O');
  await expect(work.getByRole('alert')).toHaveText(
    'The app did not say its version and OS: no answer',
  );
});

test('no_project_offers_open_with_the_macos_key', async ({ page }) => {
  await replay(page, { os: 'macos', use: ['project_last-none'] });
  await page.goto('/');
  await expect(page.locator('.tu-shell__work')).toContainText('Open a content project: ⌘O');
});

test('disabled_controls_name_their_phase', async ({ page }) => {
  await openProject(page);
  const toolbar = page.locator('.tu-toolbar');
  const expected: [RegExp, string][] = [
    [/^Write$/, 'arrives with P12'],
    [/^Source$/, 'arrives with P12'],
    [/Save/, 'arrives with P12'],
    [/New record/, 'arrives with P12'],
    [/Open in graph/, 'arrives with P15'],
  ];
  for (const [name, phase] of expected) {
    const button = toolbar.getByRole('button', { name });
    await expect(button).toBeDisabled();
    await expect(button).toHaveAttribute('title', new RegExp(phase));
  }
  await expect(toolbar.getByRole('button', { name: /^Read$/ })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await expect(toolbar.getByRole('button', { name: /Save/ })).toContainText('Ctrl+S');
  await expect(toolbar.getByRole('combobox', { name: 'Find record' })).toBeVisible();
});

test('bars_show_the_counts', async ({ page }) => {
  await openProject(page);
  const pip = page.locator('.tu-titlebar .tu-statuspip');
  await expect(pip).toHaveText('validator · 6 errors');
  await expect(pip.locator('.tu-pip')).toHaveClass(/is-reject/);
  const status = page.locator('.tu-statusbar');
  await expect(status.locator('.tu-statusbar__brand')).toHaveText('THEOS//EDITOR');
  await expect(status).toContainText('v<version> · Codex layout');
  await expect(status).toContainText('22 records · 6 errors · 2 warnings');
  await expect(status).not.toContainText('index in memory');
});

test('bars_show_an_index_in_memory', async ({ page }) => {
  await openProject(page, ['project_open-in-memory']);
  await expect(page.locator('.tu-statusbar')).toContainText(
    'index in memory: another program holds the cache',
  );
});

test('the_status_bar_follows_the_rail', async ({ page }) => {
  await openProject(page);
  await page.getByRole('navigation', { name: 'Layout presets' }).getByRole('button', { name: 'Review', exact: true }).click();
  await expect(page.locator('.tu-statusbar')).toContainText('Review layout');
});
