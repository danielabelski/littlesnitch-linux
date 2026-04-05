# Web UI Architecture

This folder contains a plain-JS single-page UI with three sections:

- `Connections`
- `Blocklists`
- `Rules`

## Files

- `index.html`
  - Static shell (tabs, section topbars, split panes, list/detail containers).
- `styles.css`
  - Shared app styles (layout, tabs, popup menus, search controls, split panes).
  - All colors and font definitions live in the `:root` block (and `html.dark` overrides) so the rest of the codebase never contains literal color or font-stack values. Variable groups: surface/text/accent colors, shadows, row/header layout tokens, font stacks (`--font-stack`, `--font-mono`, `--font-mono-code`), type scale (`--font-micro` … `--font-brand`), semantic state colors (danger, error, warning, allow/deny/mixed, search highlight, locate highlight), hover tints, modal overlays, SVG icon stroke colors, and traffic chart series colors.
  - `.is-disabled`: shared dimming class — apply to the element whose background should remain unaffected. Uses `color: var(--text-disabled)` to dim bare text nodes and `opacity: var(--disabled-opacity)` on `> *` to dim child elements (light: 0.45, dark: 0.40). Each component overrides `.is-disabled > .<actions-class>` to keep hover-only buttons hidden; the component's existing `:hover` rule restores them.
- `connections.js` / `connections.css`
  - Connections list rendering, row animations, inspector rendering.
  - Rule/blocklist links from inspector into Rules/Blocklists sections.
  - **`appendBlocklistNamesInfo(container, names)`** — shared helper (global, usable by later scripts): appends `.list-info` element(s) to `container` given an already-resolved array of blocklist name strings. Single name → inline `(name)`; multiple → `(N blocklists)` with tooltip and toggleable `<ul class="blocklist-names">`. Used by both `renderBlocklistEntry` (connections inspector) and `renderEntryRow` (blocklists combined-list entries).
- `blocklists.js` / `blocklists.css`
  - Blocklist cards (enable checkbox + name + hover edit/delete buttons + 2-line description), modals, virtualized entry list, entry selection, locate/reveal/highlight flow.
  - The combined list (id = -100) is rendered first, separated from the rest by a `.blocklist-section-separator` div; remaining lists follow in their server-provided order (user list first, then alphabetical subscribed lists).
  - Right pane contains a properties card (`renderBlocklistPropertiesCard`) followed by the entry list, both scrolling together inside `.blocklist-entry-list`.
  - Disabled blocklists get `.is-disabled` on `.blocklist-card`; disabled entries get `.is-disabled` on `.blocklist-entry-row`.
  - Combined list entries show which individual lists contain them via `appendBlocklistNamesInfo` (see `connections.js`). Names are resolved from `blocklistsById` using the `entry.blocklists` ID array (only populated for the combined list). The `.blocklist-entry-title` is a flex row; the value text lives in `.blocklist-entry-value` (with ellipsis); list info uses `.list-info`.
- `rules.js` / `rules.css`
  - Rules table rendering/sorting/selection, modal add/edit, inspector, local client-side search.
  - Disabled rules get `.is-disabled` on each `td` (added after all cells are appended via `row.cells`).
  - Inspector Action row appends ` (disabled)` when `rule.isDisabled` is true.
  - Factory rules (`rule.id < 0`) are protected in the UI: clicking Edit, Delete, or double-clicking a row shows an alert preventing modification. The bulk-delete button also checks the selection. Toggling the enabled checkbox on a factory rule shows a confirmation dialog before proceeding.
  - Shared inspector card components (`.inspector-card`, `.inspector-grid`, `.inspector-key`, `.inspector-value`, `.inspector-box`, etc.) used by both the Rules inspector and the Blocklists properties card.
- `datetime.js`
  - Unified date/time formatting. Loaded before all other scripts.
  - Reads 24h/12h preference and date separator from the system locale via `Intl.DateTimeFormat`.
  - **`window.dtPrefsOverride`** — set this object before the page loads to override locale
    detection. Fields: `hour12: bool`, `dateSep: '-'|'/'|'.'` (optional; if set, forces
    ISO component order YYYY-MM-DD with that separator; if absent, `_fmtDate` auto-detects
    component order from `Intl` and picks the conventional separator for that order:
    year-first→`-`, day-first→`.`, month-first→`/`). A future per-user settings UI will
    write here. Known limitation: POSIX `en_DK` (ISO dates) is mapped by the browser's
    Intl to day-first European order; use `{ dateSep: '-' }` as a workaround.
  - Exports (global functions available to all subsequent scripts):
    - `getDtPrefs()` — returns current `{ hour12, dateSep }` prefs object.
    - `formatDateTime(epochSeconds, showSeconds=true)` — primary formatter; returns
      `"YYYY-MM-DD HH:MM:SS"` or `"YYYY-MM-DD HH:MM"` (date separator and 12h/24h follow prefs).
    - `_fmtDate(d, prefs)` — date-only string; used by `absoluteTimeString` in `connections.js`.
    - `_fmtTime(d, prefs, showSecs)` — time-only string; used by `absoluteTimeString`.
    - `_pad(n)` — zero-pad to two digits.
- `app.js`
  - WebSocket lifecycle, global action dispatch, message routing, tabs, topbar controls, splitters.
  - Undo capsule: `undoStack` state, `handleSetUndoStack`, `renderUndoWidget`, `updateUndoAgeTick`,
    `syncUndoTimer`, `setupUndoWidget`. Receives `setUndoStack` messages. A single `.undo-capsule`
    button morphs between two states via `.is-bubble` on `.undo-widget`: expanded (accent pill +
    label, spring entry animation `undo-capsule-enter`) when newest item is < 10 s old, shrunk
    (icon-only ghost, exact `max-width: 36px` = padding + icon) otherwise. Clicking while expanded
    undoes the newest item directly; clicking while shrunk opens the `.undo-popup` dropdown.
    Animation is restarted via the remove/reflow/add trick when a new action arrives while already
    in bubble mode. Reuses `ageString()` from `connections.js` for time labels.
- `localization.js`
  - English default string table (`_strings`) + `t(key, vars?)` lookup helper.
  - **`applyLocalizationToDOM()`** — walks all `[data-i18n]` elements (sets `textContent`),
    `[data-i18n-placeholder]` elements (sets `placeholder`), `[data-i18n-aria-label]` elements
    (sets `aria-label`), `[data-i18n-alt]` elements (sets `alt`), and updates `<title>`.
    Called once at script-load time (English defaults) and again inside `setLocalizationTable`
    whenever the backend sends translated strings.
  - All static strings in `index.html` carry the appropriate `data-i18n*` attribute so a single
    `applyLocalizationToDOM()` call keeps every static text node in sync with the active locale.
  - Dynamically created elements in `traffic.js` (mode selector options, filter badge text) also
    carry `data-i18n` / `data-i18n-aria-label` so they are updated by the same call.
- `traffic.js` / `traffic.css`
  - Traffic history chart (uPlot). Renders total/received/sent bytes and blocked-connection counts
    over time. Mounted below the connections split-layout with a draggable horizontal splitter.
    Exposes `window.handleSetTrafficData`, `window.handleUpdateTrafficData`, and
    `window.rebuildTrafficPlot` (rebuilds the uPlot instance with fresh labels from `t()`);
    `app.js` calls `rebuildTrafficPlot` after receiving a `localizationTable` message.
    Manages the explicit time filter state: drag-to-zoom sends `setExplicitTimeFilter` to the
    backend and activates two UI indicators (chart badge + topbar replacement).

## Runtime Flow

1. `app.js` opens WebSocket `/stream` (or `ws://127.0.0.1:3031/stream` in file-debug mode).
2. Backend sends arrays of operations.
3. `app.js` dispatches each operation to section handlers (`connections.js`, `blocklists.js`, `rules.js`).
4. Section modules update DOM incrementally.
5. When the WebSocket closes, `app.js` shows an offline indicator in the tabs header and retries
   the connection every 10 seconds until it succeeds.

## Shared Browser State

`app.js` owns shared cross-section state:

- active section
- filter disabled state (`filterDisabled`) — synced from backend via `globalSettings`
- selected connection row ID
- selected blocklist ID
- connections sort/filter/search status (synced from backend)
- pause-updates mouse activity timers

`app.js` exposes `window.app`:

- `sendAction(type, payload?)`
- `getSelectedConnectionRowId()`
- `setSelectedConnectionRowId(rowId)`
- `getSelectedBlocklistId()`
- `setSelectedBlocklistId(blocklistId)`
- `getUserBlocklistId()`
- `getConnectionsSort()` — returns the current sort key string (e.g. `"totalDataSent"`)
- `setConnectionsSort(key)` — updates `state.connectionsSort` immediately (call before `applyConnectionsSort`)

Extra globals used for cross-section navigation and keyboard handling:

- `window.selectRuleInRulesSection(ruleId)` (`rules.js`)
- `window.setRulesSearchQuery(query)` (`rules.js`)
- `window.selectBlocklistEntryInBlocklist(entryType, value, blocklistId)` (`blocklists.js`)
- `window.navigateConnectionsSelection(delta)` (`connections.js`) — move selection ±1 row; called by `app.js` keyboard handler
- `window.maybeToggleConnectionDisclosureForKey(key)` (`connections.js`) — handle arrow/space disclosure toggle; called by `app.js` keyboard handler
- `window.applyConnectionsSort()` (`connections.js`) — re-renders the `#connections-header` bar and all `.total-bytes` spans from cached rx/tx values; called by `app.js` after any sort state change (both user-initiated and from backend)

## Browser -> Backend Actions

All actions are sent as JSON with an `action` field.

Sent by `app.js`:

- `setSection`
- `setFilterDisabled` — sent when the user toggles the header filter switch; payload: `{ filterDisabled: bool }`
- `undo` — sent when the user clicks the bubble or a dropdown row; payload: `{ itemId }`
- `setSearch` (Connections, Blocklists; Rules search is local in `rules.js`)
- `setConnectionsSort` — also sent by `connections.js` header click handlers
- `setConnectionsFilters`
- `pauseUpdates`

Sent by `connections.js`:

- `toggleDisclosure`
- `selectRow`
- `toggleRule`
- `setRuleDisabled`
- `setBlocklistEntryDisabled`

Sent by `blocklists.js`:

- `setBlocklistFilter` — sent when the "Show disabled entries only" checkbox changes; payload: `{ disabledEntriesOnly: bool }`
- `setSearch`
- `selectBlocklist`
- `loadBlocklistEntries`
- `locateBlocklistEntry`
- `setBlocklistEntryDisabled`
- `addBlocklist`
- `editBlocklist`
- `deleteBlocklist`
- `addUserBlocklistEntries`
- `removeUserBlocklistEntries`

Sent by `traffic.js`:

- `setExplicitTimeFilter` — sent when the user drag-zooms the chart (sets `startSecs` /
  `endInclusiveSecs`) or cancels the filter (both fields `null`).

Sent by `rules.js`:

- `setRuleDisabled`
- `addRule`
- `editRule`
- `deleteRules`

## Backend -> Browser Operations

Handled by `app.js` router:

- `clearConnectionRows`
- `insertConnectionRows`
- `removeConnectionRows`
- `moveConnetionRows`
- `updateConnectionRows`
- `updateRuleButtons`
- `highlightRuleForRows` — triggers a ~1 s bounce animation on the rule or details-differ button of each row in `ids`; scrolls the first row into view first; field: `ids` (array), `action`; handled by `highlightRuleButtons()` in `connections.js`
- `trafficEvents`
- `setInspector`
- `setBlocklists`
- `setRules`
- `updateRules`
- `setBlocklistDetails`
- `setBlocklistEntries`
- `setBlocklistEntryLocation`
- `setBlocklistStatus` — syncs blocklist filter state; fields: `searchTerm`, `disabledEntriesOnly`; handled in `app.js` by `handleSetBlocklistStatus()`
- `setConnectionsStatus`
- `setTrafficData`
- `updateTrafficData`
- `setAboutInfo` — populates the About dialog (version, commits, copyright, website URL); handled in `app.js` by `handleSetAboutInfo()`
- `setUndoStack` — updates the undo stack shown in the header; handled in `app.js` by `handleSetUndoStack()`
- `localizationTable` — sends a key→string map that overrides English defaults; handled in `app.js`
  by `setLocalizationTable()` (`localization.js`), which merges the table, then calls
  `applyLocalizationToDOM()`, `window.applyConnectionsSort()`, and `window.rebuildTrafficPlot()`
- `globalSettings` — sets global state; currently carries `filterDisabled: bool`; handled in `app.js` by `handleSetGlobalSettings()`, which updates `state.filterDisabled` and the header filter switch

## Section Notes

- Connections
  - Filters are inline `<select>` controls in topbar. The location filter (`data-role="location-filter"`) is a single control that encodes both `localnet` and `localhost` fields. Its five primary options are: `all` (Internet + Local Networks; `localnet=null, localhost=false`), `internet` (`localnet=false, localhost=false`), `localnet` (`localnet=true, localhost=false`), `localhost` (`localnet=false, localhost=true`), `everything` (`localnet=null, localhost=null`). Two temporary options (`internet-and-localhost`, `invalid`) are appended dynamically when the backend sends a combination that doesn't match a primary option; they are removed as soon as a standard option becomes active.
  - Sort is driven by the `#connections-header` bar (columns: Connection, Rule, Traffic, Activity). Clicking a sortable column sends `setConnectionsSort` to the backend; sort state is owned by the backend and echoed back via `setConnectionsStatus`, which triggers `refreshConnectionsHeader()`. The Traffic column header shows a popup (Total Traffic / Bytes In / Bytes Out) and its label changes to "Bytes In" or "Bytes Out" when the corresponding sort is active. Sort directions are fixed: Connection ▲ (name asc), Traffic ▼ (total/rx/tx desc), Activity ▲ (lastActivity asc). Rule column is display-only. The shared `window.app.showSortPopup()` utility (in `app.js`) powers both the Traffic column popup and the Rules section's last-column popup.
  - Each row shows two statistics spans: `.total-bytes` (SI units, 3 sig-figs; shows rx+tx, tx-only, or rx-only depending on sort) and `.last-event` (age string with 10 s resolution; green/red rounded background when allow/deny is clearly dominant).
  - "More items" rows (`isMoreItems=true`) receive `updateRuleButtons` data from the backend. They render a `.rule-button-placeholder` (24 px, `margin-left: auto`) instead of the rule button, plus a `.details-button` that is shown/hidden exactly like regular rows. Clicking the details-differ button on a more-items row sends `toggleDisclosure` with `expandToDifferingDetail: true`.
  - `pauseUpdates` is sent every 2 s while the mouse is active inside the window and Connections is active; stops on leave/idle/background.

- Blocklists
  - Right pane: a properties card (`.inspector-card.blocklist-properties-card`) is inserted as the first child of `.blocklist-entry-list` inside a `headerEl` wrapper div, so it scrolls together with the entries. The `activeVirtualList` object carries a `headerEl` reference.
  - Virtual scroll math in `renderVirtualListRows` and `centerVirtualListOnIndex` subtracts/adds `headerEl.offsetHeight` so row index → pixel mapping stays correct.
  - Properties card is built by `renderBlocklistPropertiesCard(blocklist)` using the shared `appendInspectorRow` / `appendInspectorBox` helpers from `rules.js`. Calls use `{ plain: true }` to avoid rules-search highlighting leaking into blocklist values.
  - Locate flow for connection-linked entries uses backend index lookup (`locateBlocklistEntry` -> `setBlocklistEntryLocation`) and row highlight.
  - Card animations: `handleSetBlocklists` diffs incoming IDs against `previousBlocklistIds`. Removed cards get a 250 ms flash (`.is-card-removing`) before the DOM rebuild; new cards get a 250 ms slide-fade-in (`.is-card-added`). First render (when `previousBlocklistIds` is `null`) skips animations. A `blocklistUpdateTimer` cancels pending removal timeouts on rapid updates.

- Traffic Chart (bottom of Connections section)
  - Rendered by `traffic.js` using uPlot below the connections split-layout.
  - Four series: Total bytes (blue, filled), Received (green), Sent (orange) share the left
    bytes Y-axis; Blocked count (red dashed) uses an independent right Y-axis.
  - `SetTrafficData`: replaces all chart data. Provides `timeQuantum` (seconds per slot),
    `startTime` (in quanta), and equal-length arrays `bytesReceived`, `bytesSent`, `blockCount`.
    Timestamps for index `i` are `(startTime + i) * timeQuantum` seconds since Unix epoch.
  - `UpdateTrafficData`: streaming update. `startTime` defines the new window start (entries
    before it are pruned). `updatedTime` identifies the slot to overwrite (last slot) or append.
    `timeQuantum` is unchanged from the last `SetTrafficData`.
  - Click-drag on the plot zooms the x-axis and sends `setExplicitTimeFilter` to the backend
    with the inclusive second range. Double-click resets the chart zoom (visual only; does not
    clear the backend filter — use the "×" buttons for that).
  - When a time filter is active, two indicators appear:
    - A `.traffic-filter-badge` overlay in the bottom-left of the chart with an "×" cancel button.
    - The `[data-role="visible-period-filter"]` select in the topbar is hidden and replaced by a
      `.time-filter-indicator` box with an "×" cancel button.
  - The horizontal splitter between the split-layout and the chart is managed by `traffic.js`.
  - X-axis tick labels use a custom `fmtDate` factory (`uplotFmtDate` / `fmtDatePart` in
    `traffic.js`) that delegates to `datetime.js` helpers, so the locale date order and
    12 h/24 h preference are honoured while uPlot's tick-granularity algorithm is unchanged.

- Rules
  - Search is frontend-only and filters local rule list.
  - Search highlights matches in list/inspector content.
  - Table supports multi-sort via clickable headers and multi-selection semantics.
  - The left pane has no `pane-title` bar. The "Add rule" (`+`) and "Delete selected rules" (`-`) buttons are rendered inside a `.rule-th-actions` flex wrapper in the last `<th>` of the table header (the actions column) using class `blocklist-add-button`. The `-` button (`data-role="delete-selected-rules"`) is disabled when no rules are selected; `refreshRuleSelectionStyles` keeps its disabled state in sync.
  - The Port column (column 6, `sortPopup: true`) acts as a sort chooser instead of a toggle: clicking its header opens a `.rules-sort-popup` with five fixed-direction options: Port and Protocol (asc), Modified (desc), Created (desc), Precedence (asc), Priority (desc). State is tracked in `rulesLastColumnSort` (default `'port'`). When a non-port option is active, its name replaces "Port" as the column header text. Original server order (for Precedence sort) is stored in `rulesOriginalOrder` Map (rule.id → index), rebuilt each time `applyRulesData` runs.
  - Row animations: `handleUpdateRules` adds a 250 ms removal flash (`.is-row-removing`) on removed rows before the DOM rebuild, then marks inserted rows with `.is-row-added` (300 ms fade-in + accent highlight). A `rulesUpdateTimer` guards against overlapping updates — a new update cancels any pending removal timeout. Initial load (`handleSetRules`) and local search/sort changes skip animations.

## Contract Source of Truth

UiUpdate/action payload shapes are defined in Rust:

- `daemon/src/client/ui_types.rs`
