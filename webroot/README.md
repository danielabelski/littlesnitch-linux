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
- `connections.js` / `connections.css`
  - Connections list rendering, row animations, inspector rendering.
  - Rule/blocklist links from inspector into Rules/Blocklists sections.
- `blocklists.js` / `blocklists.css`
  - Blocklist cards, modals, virtualized entry list, entry selection, locate/reveal/highlight flow.
- `rules.js` / `rules.css`
  - Rules table rendering/sorting/selection, modal add/edit, inspector, local client-side search.
- `app.js`
  - WebSocket lifecycle, global action dispatch, message routing, tabs, topbar controls, splitters.
- `traffic.js` / `traffic.css`
  - Traffic history chart (uPlot). Renders total/received/sent bytes and blocked-connection counts
    over time. Mounted below the connections split-layout with a draggable horizontal splitter.
    Exposes `window.handleSetTrafficData` and `window.handleUpdateTrafficData`; called by `app.js`.
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

Extra globals used for cross-section navigation and keyboard handling:

- `window.selectRuleInRulesSection(ruleId)` (`rules.js`)
- `window.setRulesSearchQuery(query)` (`rules.js`)
- `window.selectBlocklistEntryInBlocklist(entryType, value, blocklistId)` (`blocklists.js`)
- `window.navigateConnectionsSelection(delta)` (`connections.js`) — move selection ±1 row; called by `app.js` keyboard handler
- `window.maybeToggleConnectionDisclosureForKey(key)` (`connections.js`) — handle arrow/space disclosure toggle; called by `app.js` keyboard handler
- `window.refreshConnectionsBytes()` (`connections.js`) — re-render all `.total-bytes` spans from cached rx/tx values; called by `app.js` when sort changes

## Browser -> Backend Actions

All actions are sent as JSON with an `action` field.

Sent by `app.js`:

- `setSection`
- `setSearch` (Connections, Blocklists; Rules search is local in `rules.js`)
- `setConnectionsSort`
- `setConnectionsFilters`
- `pauseUpdates`

Sent by `connections.js`:

- `toggleDisclosure`
- `selectRow`
- `toggleRule`
- `toggleRuleEnabled`
- `toggleBlocklistEntryEnabled`

Sent by `blocklists.js`:

- `setSearch`
- `selectBlocklist`
- `loadBlocklistEntries`
- `locateBlocklistEntry`
- `toggleBlocklistEntryEnabled`
- `addBlocklist`
- `editBlocklist`
- `deleteBlocklist`
- `addUserBlocklistEntries`
- `removeUserBlocklistEntries`

Sent by `traffic.js`:

- `setExplicitTimeFilter` — sent when the user drag-zooms the chart (sets `start_secs` /
  `end_inclusive_secs`) or cancels the filter (both fields `null`).

Sent by `rules.js`:

- `toggleRuleEnabled`
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
- `trafficEvents`
- `setInspector`
- `setBlocklists`
- `setRules`
- `updateRules`
- `setBlocklistDetails`
- `setBlocklistEntries`
- `setBlocklistEntryLocation`
- `setConnectionsStatus`
- `setTrafficData`
- `updateTrafficData`
- `setAboutInfo` — populates the About dialog (version, commits, copyright, website URL); handled in `app.js` by `handleSetAboutInfo()`

## Section Notes

- Connections
  - Filters are inline `<select>` controls in topbar. The location filter (`data-role="location-filter"`) is a single control that encodes both `localnet` and `localhost` fields. Its five primary options are: `all` (Internet + Local Networks; `localnet=null, localhost=false`), `internet` (`localnet=false, localhost=false`), `localnet` (`localnet=true, localhost=false`), `localhost` (`localnet=false, localhost=true`), `everything` (`localnet=null, localhost=null`). Two temporary options (`internet-and-localhost`, `invalid`) are appended dynamically when the backend sends a combination that doesn't match a primary option; they are removed as soon as a standard option becomes active.
  - Sort is a `<select>` dropdown (`data-role="connections-sort"`); sort state is owned by the backend and echoed back via `setConnectionsStatus`.
  - Each row shows two statistics spans: `.total-bytes` (SI units, 3 sig-figs; shows rx+tx, tx-only, or rx-only depending on sort) and `.last-event` (age string with 10 s resolution; green/red rounded background when allow/deny is clearly dominant).
  - `pauseUpdates` is sent every 2 s while the mouse is active inside the window and Connections is active; stops on leave/idle/background.

- Blocklists
  - Right pane uses virtual scrolling (`loadBlocklistEntries` windowing).
  - Locate flow for connection-linked entries uses backend index lookup (`locateBlocklistEntry` -> `setBlocklistEntryLocation`) and row highlight.

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

- Rules
  - Search is frontend-only and filters local rule list.
  - Search highlights matches in list/inspector content.
  - Table supports multi-sort via clickable headers and multi-selection semantics.

## Contract Source of Truth

UiUpdate/action payload shapes are defined in Rust:

- `daemon/src/client/ui_types.rs`
