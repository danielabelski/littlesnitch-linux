// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

const VIRTUAL_ROW_HEIGHT = 24;
const VIRTUAL_OVERSCAN = 20;
const VIRTUAL_PAGE_SIZE = 240;
const VIRTUAL_MAX_IN_FLIGHT = 2;
const detailsState = new Map(); // blocklistId -> { totalEntries, entries: Map<index, entry>, pending: Set<rangeKey> }
const blocklistsById = new Map(); // blocklistId -> blocklist
let activeVirtualList = null;
let activeDetailsBlocklistId = null;
let addBlocklistDialog = null;
let addBlocklistError = null;
let addBlocklistName = null;
let addBlocklistDescription = null;
let addBlocklistUrl = null;
let addBlocklistNamesAreHosts = null;
let addBlocklistUpdatePeriod = null;
let addBlocklistTitle = null;
let addBlocklistConfirm = null;
let editBlocklistId = null;
let addUserEntriesDialog = null;
let addUserEntriesError = null;
let addUserEntriesText = null;
let addUserEntriesNamesAreHosts = null;
const selectedUserEntries = new Map(); // key -> { entryType, value }
let userSelectionAnchorIndex = null;
let highlightedEntry = null; // { blocklistId, entryType, value }
let pendingLocateEntry = null; // { blocklistId, entryType, value, index, inFlight }
const UPDATE_PERIOD_PRESET_OPTIONS = [
  { minutes: 60, label: 'every hour' },
  { minutes: 360, label: 'every 6 hours' },
  { minutes: 1440, label: 'every day' },
  { minutes: 10080, label: 'every week' },
];

function getUserBlocklistId() {
  if (window.app && typeof window.app.getUserBlocklistId === 'function') {
    return window.app.getUserBlocklistId();
  }
  return -1;
}

function ensureBlocklistDetails(blocklistId) {
  let details = detailsState.get(blocklistId);
  if (!details) {
    details = {
      totalEntries: 0,
      entries: new Map(),
      pending: new Set(),
    };
    detailsState.set(blocklistId, details);
  }
  return details;
}

function getBlocklistsDetailsContainer() {
  return document.querySelector('.section[data-section="blocklists"] [data-role="details"]');
}

function getBlocklistsSearchQuery() {
  const input = document.querySelector('.section[data-section="blocklists"] [data-role="search"]');
  if (!input) {
    return '';
  }
  return input.value.trim();
}

function setBlocklistsSearchQuery(query) {
  const input = document.querySelector('.section[data-section="blocklists"] [data-role="search"]');
  if (!input) {
    return;
  }
  const nextQuery = query || '';
  input.value = nextQuery;
  input.classList.toggle('is-filtered', nextQuery.trim().length > 0);
  if (window.app && typeof window.app.sendAction === 'function') {
    window.app.sendAction('setSearch', { query: nextQuery });
  }
}

function getEmptyEntriesText() {
  return getBlocklistsSearchQuery().length > 0 ? 'No matching entries' : 'No entries';
}

function getSelectedBlocklistId() {
  if (!window.app || typeof window.app.getSelectedBlocklistId !== 'function') {
    return getUserBlocklistId();
  }
  const id = window.app.getSelectedBlocklistId();
  return id === null || id === undefined ? getUserBlocklistId() : id;
}

function setSelectedBlocklistId(blocklistId) {
  if (!window.app || typeof window.app.setSelectedBlocklistId !== 'function') {
    return;
  }
  window.app.setSelectedBlocklistId(blocklistId);
}

function entryMatchesEntryRef(entry, entryRef) {
  return !!entry && !!entryRef && entry.entryType === entryRef.entryType && entry.value === entryRef.value;
}

function requestLocateBlocklistEntry() {
  if (!pendingLocateEntry || pendingLocateEntry.inFlight) {
    return;
  }
  if (!window.app || typeof window.app.sendAction !== 'function') {
    return;
  }
  pendingLocateEntry.inFlight = true;
  window.app.sendAction('locateBlocklistEntry', {
    blocklistId: pendingLocateEntry.blocklistId,
    entryType: pendingLocateEntry.entryType,
    value: pendingLocateEntry.value,
  });
}

function centerVirtualListOnIndex(ctx, index) {
  if (!ctx || !ctx.listEl || !Number.isFinite(index) || index < 0) {
    return;
  }
  const targetTop = Math.max(
    0,
    (index * VIRTUAL_ROW_HEIGHT)
      - (ctx.listEl.clientHeight / 2)
      + (VIRTUAL_ROW_HEIGHT / 2),
  );
  ctx.listEl.scrollTop = targetTop;
  renderVirtualListRows(ctx);
}

function requestEntryRangeAroundIndex(blocklistId, index) {
  if (!Number.isFinite(index) || index < 0) {
    return;
  }
  if (!window.app || typeof window.app.sendAction !== 'function') {
    return;
  }
  const start = Math.max(0, index - Math.floor(VIRTUAL_PAGE_SIZE / 2));
  window.app.sendAction('loadBlocklistEntries', {
    blocklistId,
    start,
    limit: VIRTUAL_PAGE_SIZE,
  });
}

function tryCompletePendingLocate() {
  if (!pendingLocateEntry) {
    return;
  }
  if (getSelectedBlocklistId() !== pendingLocateEntry.blocklistId) {
    return;
  }
  if (!Number.isFinite(pendingLocateEntry.index)) {
    requestLocateBlocklistEntry();
    return;
  }

  if (activeVirtualList && activeVirtualList.blocklistId === pendingLocateEntry.blocklistId) {
    centerVirtualListOnIndex(activeVirtualList, pendingLocateEntry.index);
  }

  const details = ensureBlocklistDetails(pendingLocateEntry.blocklistId);
  const entryAtIndex = details.entries.get(pendingLocateEntry.index);
  if (!entryMatchesEntryRef(entryAtIndex, pendingLocateEntry)) {
    requestEntryRangeAroundIndex(pendingLocateEntry.blocklistId, pendingLocateEntry.index);
    return;
  }

  highlightedEntry = {
    blocklistId: pendingLocateEntry.blocklistId,
    entryType: pendingLocateEntry.entryType,
    value: pendingLocateEntry.value,
  };
  pendingLocateEntry = null;
  refreshVirtualList();
}

function selectBlocklistEntryInBlocklist(entryType, value, blocklistId) {
  const targetBlocklistId = Number(blocklistId);
  if (!entryType || !value || !Number.isFinite(targetBlocklistId)) {
    return;
  }
  highlightedEntry = null;
  pendingLocateEntry = {
    blocklistId: targetBlocklistId,
    entryType,
    value,
    index: null,
    inFlight: false,
  };
  const blocklistsTab = document.querySelector('.tab[data-section="blocklists"]');
  if (blocklistsTab instanceof HTMLButtonElement) {
    blocklistsTab.click();
  }
  if (getSelectedBlocklistId() !== targetBlocklistId) {
    setSelectedBlocklistId(targetBlocklistId);
    if (window.app && typeof window.app.sendAction === 'function') {
      window.app.sendAction('selectBlocklist', { id: targetBlocklistId });
    }
  }
  renderBlocklistDetails();
  requestLocateBlocklistEntry();
}

window.selectBlocklistEntryInBlocklist = selectBlocklistEntryInBlocklist;

function handleSetBlocklistEntryLocation(msg) {
  if (!pendingLocateEntry) {
    return;
  }

  if (
    pendingLocateEntry.blocklistId !== msg.blocklistId
    || pendingLocateEntry.entryType !== msg.entryType
    || pendingLocateEntry.value !== msg.value
  ) {
    return;
  }

  pendingLocateEntry.inFlight = false;

  if (msg.clearSearch) {
    setBlocklistsSearchQuery('');
  }

  const index = msg.index;
  if (index === null || index === undefined) {
    pendingLocateEntry = null;
    return;
  }

  pendingLocateEntry.index = Number(index);
  renderBlocklistDetails();
  tryCompletePendingLocate();
}

function setAddBlocklistError(message) {
  if (!addBlocklistError) {
    return;
  }
  addBlocklistError.textContent = message || '';
}

function setAddUserEntriesError(message) {
  if (!addUserEntriesError) {
    return;
  }
  addUserEntriesError.textContent = message || '';
}

function submitAddUserEntriesModal() {
  if (!addUserEntriesDialog || !addUserEntriesText || !addUserEntriesNamesAreHosts) {
    return;
  }
  const selectedId = getSelectedBlocklistId();
  if (selectedId !== getUserBlocklistId()) {
    addUserEntriesDialog.close();
    return;
  }
  const entries = addUserEntriesText.value
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  if (entries.length === 0) {
    setAddUserEntriesError('Add at least one entry.');
    addUserEntriesText.focus();
    return;
  }
  const namesAreHosts = addUserEntriesNamesAreHosts.checked;
  if (window.app && typeof window.app.sendAction === 'function') {
    window.app.sendAction('addUserBlocklistEntries', { entries, namesAreHosts });
  }
  addUserEntriesDialog.close();
}

function ensureUserEntriesDialog() {
  if (addUserEntriesDialog) {
    return addUserEntriesDialog;
  }

  const dialog = document.createElement('dialog');
  dialog.className = 'blocklist-modal';

  const form = document.createElement('form');
  form.className = 'blocklist-modal-form';
  form.method = 'dialog';
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    submitAddUserEntriesModal();
  });

  const title = document.createElement('h2');
  title.className = 'blocklist-modal-title';
  title.textContent = 'Add Blocklist Entries';
  form.appendChild(title);

  const entriesLabel = document.createElement('label');
  entriesLabel.className = 'blocklist-modal-label';
  entriesLabel.textContent = 'Entries (one per line)';
  const entriesInput = document.createElement('textarea');
  entriesInput.className = 'blocklist-modal-textarea';
  entriesInput.name = 'entries';
  entriesInput.rows = 10;
  entriesLabel.appendChild(entriesInput);
  form.appendChild(entriesLabel);

  const namesAreHostsLabel = document.createElement('label');
  namesAreHostsLabel.className = 'blocklist-modal-checkbox-label';
  const namesAreHostsInput = document.createElement('input');
  namesAreHostsInput.type = 'checkbox';
  namesAreHostsInput.name = 'namesAreHosts';
  namesAreHostsLabel.appendChild(namesAreHostsInput);
  namesAreHostsLabel.appendChild(document.createTextNode(' names are hosts, not domains'));
  form.appendChild(namesAreHostsLabel);

  const error = document.createElement('div');
  error.className = 'blocklist-modal-error';
  form.appendChild(error);

  const actions = document.createElement('div');
  actions.className = 'blocklist-modal-actions';

  const cancelButton = document.createElement('button');
  cancelButton.type = 'button';
  cancelButton.className = 'blocklist-modal-button';
  cancelButton.textContent = 'Cancel';
  cancelButton.addEventListener('click', () => dialog.close());
  actions.appendChild(cancelButton);

  const addButton = document.createElement('button');
  addButton.type = 'submit';
  addButton.className = 'blocklist-modal-button is-primary';
  addButton.textContent = 'Add';
  actions.appendChild(addButton);

  form.appendChild(actions);
  dialog.appendChild(form);

  dialog.addEventListener('close', () => {
    setAddUserEntriesError('');
    form.reset();
  });

  document.body.appendChild(dialog);

  addUserEntriesDialog = dialog;
  addUserEntriesError = error;
  addUserEntriesText = entriesInput;
  addUserEntriesNamesAreHosts = namesAreHostsInput;
  return dialog;
}

function openUserEntriesModal() {
  const dialog = ensureUserEntriesDialog();
  setAddUserEntriesError('');
  dialog.showModal();
  if (addUserEntriesText) {
    addUserEntriesText.focus();
  }
}

function isUserBlocklistSelected() {
  return getSelectedBlocklistId() === getUserBlocklistId();
}

function entrySelectionKey(entryType, value) {
  return `${entryType}\n${value}`;
}

function clearSelectedUserEntries() {
  selectedUserEntries.clear();
  userSelectionAnchorIndex = null;
}

function submitBlocklistModal() {
  if (
    !addBlocklistDialog
    || !addBlocklistName
    || !addBlocklistDescription
    || !addBlocklistUrl
    || !addBlocklistNamesAreHosts
  ) {
    return;
  }

  const name = addBlocklistName.value.trim();
  const description = addBlocklistDescription.value.trim();
  const updateFromUrl = addBlocklistUrl.value.trim();
  const namesAreHosts = addBlocklistNamesAreHosts.checked;
  const updatePeriodMinutes = addBlocklistUpdatePeriod
    ? Number(addBlocklistUpdatePeriod.value)
    : 1440;

  if (name.length === 0) {
    setAddBlocklistError('Name is required.');
    addBlocklistName.focus();
    return;
  }
  if (updateFromUrl.length === 0) {
    setAddBlocklistError('URL is required.');
    addBlocklistUrl.focus();
    return;
  }

  try {
    const parsedUrl = new URL(updateFromUrl);
    if (parsedUrl.protocol !== 'http:' && parsedUrl.protocol !== 'https:') {
      throw new Error('Only HTTP(S) URLs are supported.');
    }
  } catch (_error) {
    setAddBlocklistError('Please enter a valid HTTP(S) URL.');
    addBlocklistUrl.focus();
    return;
  }
  if (!Number.isFinite(updatePeriodMinutes) || updatePeriodMinutes <= 0) {
    setAddBlocklistError('Please select a valid update period.');
    addBlocklistUpdatePeriod?.focus();
    return;
  }

  if (window.app && typeof window.app.sendAction === 'function') {
    if (editBlocklistId === null || editBlocklistId === undefined) {
      window.app.sendAction('addBlocklist', {
        name,
        description,
        updateFromUrl,
        namesAreHosts,
        updatePeriodMinutes,
      });
    } else {
      window.app.sendAction('editBlocklist', {
        blocklistId: editBlocklistId,
        name,
        description,
        updateFromUrl,
        namesAreHosts,
        updatePeriodMinutes,
      });
    }
  }
  addBlocklistDialog.close();
}

function ensureAddBlocklistDialog() {
  if (addBlocklistDialog) {
    return addBlocklistDialog;
  }

  const dialog = document.createElement('dialog');
  dialog.className = 'blocklist-modal';

  const form = document.createElement('form');
  form.className = 'blocklist-modal-form';
  form.method = 'dialog';
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    submitBlocklistModal();
  });

  const title = document.createElement('h2');
  title.className = 'blocklist-modal-title';
  title.textContent = 'Add Blocklist';
  form.appendChild(title);

  const nameLabel = document.createElement('label');
  nameLabel.className = 'blocklist-modal-label';
  nameLabel.textContent = 'Name';
  const nameInput = document.createElement('input');
  nameInput.className = 'blocklist-modal-input';
  nameInput.type = 'text';
  nameInput.name = 'name';
  nameInput.required = true;
  nameLabel.appendChild(nameInput);
  form.appendChild(nameLabel);

  const descriptionLabel = document.createElement('label');
  descriptionLabel.className = 'blocklist-modal-label';
  descriptionLabel.textContent = 'Description';
  const descriptionInput = document.createElement('textarea');
  descriptionInput.className = 'blocklist-modal-textarea';
  descriptionInput.name = 'description';
  descriptionInput.rows = 3;
  descriptionLabel.appendChild(descriptionInput);
  form.appendChild(descriptionLabel);

  const urlLabel = document.createElement('label');
  urlLabel.className = 'blocklist-modal-label';
  urlLabel.textContent = 'URL';
  const urlInput = document.createElement('input');
  urlInput.className = 'blocklist-modal-input';
  urlInput.type = 'url';
  urlInput.name = 'url';
  urlInput.required = true;
  urlInput.placeholder = 'https://example.com/blocklist.txt';
  urlLabel.appendChild(urlInput);
  form.appendChild(urlLabel);

  const namesAreHostsLabel = document.createElement('label');
  namesAreHostsLabel.className = 'blocklist-modal-checkbox-label';
  const namesAreHostsInput = document.createElement('input');
  namesAreHostsInput.type = 'checkbox';
  namesAreHostsInput.name = 'namesAreHosts';
  namesAreHostsLabel.appendChild(namesAreHostsInput);
  namesAreHostsLabel.appendChild(document.createTextNode(' Treat as list of hostnames'));
  form.appendChild(namesAreHostsLabel);

  const updatePeriodLabel = document.createElement('label');
  updatePeriodLabel.className = 'blocklist-modal-label';
  updatePeriodLabel.textContent = 'Update Period';
  const updatePeriodSelect = document.createElement('select');
  updatePeriodSelect.className = 'blocklist-modal-select';
  updatePeriodSelect.name = 'updatePeriodMinutes';
  updatePeriodLabel.appendChild(updatePeriodSelect);
  form.appendChild(updatePeriodLabel);

  const error = document.createElement('div');
  error.className = 'blocklist-modal-error';
  form.appendChild(error);

  const actions = document.createElement('div');
  actions.className = 'blocklist-modal-actions';

  const cancelButton = document.createElement('button');
  cancelButton.type = 'button';
  cancelButton.className = 'blocklist-modal-button';
  cancelButton.textContent = 'Cancel';
  cancelButton.addEventListener('click', () => dialog.close());
  actions.appendChild(cancelButton);

  const addButton = document.createElement('button');
  addButton.type = 'submit';
  addButton.className = 'blocklist-modal-button is-primary';
  addButton.textContent = 'Add';
  actions.appendChild(addButton);

  form.appendChild(actions);
  dialog.appendChild(form);

  dialog.addEventListener('close', () => {
    setAddBlocklistError('');
    editBlocklistId = null;
    form.reset();
  });

  document.body.appendChild(dialog);

  addBlocklistDialog = dialog;
  addBlocklistError = error;
  addBlocklistName = nameInput;
  addBlocklistDescription = descriptionInput;
  addBlocklistUrl = urlInput;
  addBlocklistNamesAreHosts = namesAreHostsInput;
  addBlocklistUpdatePeriod = updatePeriodSelect;
  addBlocklistTitle = title;
  addBlocklistConfirm = addButton;
  setUpdatePeriodOptions(1440);
  return dialog;
}

function openBlocklistModal(blocklist) {
  const dialog = ensureAddBlocklistDialog();
  setAddBlocklistError('');
  if (blocklist) {
    editBlocklistId = blocklist.id;
    addBlocklistTitle.textContent = 'Edit Blocklist';
    addBlocklistConfirm.textContent = 'Save';
    addBlocklistName.value = blocklist.name || '';
    addBlocklistDescription.value = blocklist.description || '';
    addBlocklistUrl.value = blocklist.updateFromUrl || '';
    addBlocklistNamesAreHosts.checked = blocklist.namesAreHosts === true;
    setUpdatePeriodOptions(blocklist.updatePeriodMinutes);
  } else {
    editBlocklistId = null;
    addBlocklistTitle.textContent = 'Add Blocklist';
    addBlocklistConfirm.textContent = 'Add';
    addBlocklistDialog.querySelector('form')?.reset();
    setUpdatePeriodOptions(1440);
  }
  dialog.showModal();
  if (addBlocklistName) {
    addBlocklistName.focus();
  }
}

function setupBlocklistHeaderAddButton() {
  const title = document.querySelector('.section[data-section="blocklists"] .left-pane .pane-title');
  if (!title || title.querySelector('[data-role="add-blocklist"]')) {
    return;
  }

  title.classList.add('blocklist-pane-title');

  const label = document.createElement('span');
  label.textContent = title.textContent || 'Blocklist List';
  title.textContent = '';
  title.appendChild(label);

  const addButton = document.createElement('button');
  addButton.type = 'button';
  addButton.className = 'blocklist-add-button';
  addButton.setAttribute('data-role', 'add-blocklist');
  addButton.setAttribute('aria-label', 'Add blocklist');
  addButton.title = 'Add blocklist';
  addButton.textContent = '+';
  addButton.addEventListener('click', () => {
    openBlocklistModal(null);
  });
  title.appendChild(addButton);
}

function setupBlocklistDetailsHeaderAddButton() {
  const title = document.querySelector('.section[data-section="blocklists"] .right-pane .pane-title');
  if (!title) {
    return;
  }

  let addButton = title.querySelector('[data-role="add-user-entries"]');
  let removeButton = title.querySelector('[data-role="remove-user-entries"]');
  let actions = title.querySelector('[data-role="user-entries-actions"]');
  if (!addButton || !removeButton) {
    title.classList.add('blocklist-pane-title');

    const label = document.createElement('span');
    label.textContent = title.textContent || 'Blocklist Details';
    title.textContent = '';
    title.appendChild(label);

    actions = document.createElement('div');
    actions.className = 'blocklist-pane-actions';
    actions.setAttribute('data-role', 'user-entries-actions');
    title.appendChild(actions);

    addButton = document.createElement('button');
    addButton.type = 'button';
    addButton.className = 'blocklist-add-button';
    addButton.setAttribute('data-role', 'add-user-entries');
    addButton.setAttribute('aria-label', 'Add entries');
    addButton.title = 'Add entries';
    addButton.textContent = '+';
    addButton.hidden = true;
    addButton.addEventListener('click', () => {
      openUserEntriesModal();
    });
    actions.appendChild(addButton);

    removeButton = document.createElement('button');
    removeButton.type = 'button';
    removeButton.className = 'blocklist-add-button';
    removeButton.setAttribute('data-role', 'remove-user-entries');
    removeButton.setAttribute('aria-label', 'Remove selected entries');
    removeButton.title = 'Remove selected entries';
    removeButton.textContent = '-';
    removeButton.hidden = true;
    removeButton.disabled = true;
    removeButton.addEventListener('click', () => {
      if (selectedUserEntries.size === 0) {
        return;
      }
      if (window.app && typeof window.app.sendAction === 'function') {
        window.app.sendAction('removeUserBlocklistEntries', {
          entries: Array.from(selectedUserEntries.values()),
        });
      }
      clearSelectedUserEntries();
      refreshVirtualList();
      setupBlocklistDetailsHeaderAddButton();
    });
    actions.appendChild(removeButton);
  }

  const visible = isUserBlocklistSelected();
  addButton.hidden = !visible;
  removeButton.hidden = !visible;
  removeButton.disabled = selectedUserEntries.size === 0;
}

function formatEntryTitle(entry) {
  const value = entry.value || '';
  if (entry.entryType === 'domain') {
    return `domain ${value}`;
  }
  return value;
}

function formatLocalTime(secondsSinceEpoch) {
  const seconds = Number(secondsSinceEpoch);
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return '';
  }
  const date = new Date(seconds * 1000);
  if (Number.isNaN(date.getTime())) {
    return '';
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'medium',
  }).format(date);
}

function normalizeUpdatePeriodMinutes(value) {
  const minutes = Number(value);
  if (!Number.isFinite(minutes) || minutes <= 0) {
    return 1440;
  }
  return Math.max(1, Math.round(minutes));
}

function formatUpdatePeriodForDisplay(minutesValue) {
  const minutes = normalizeUpdatePeriodMinutes(minutesValue);
  if (minutes === 60) {
    return 'every hour';
  }
  if (minutes === 360) {
    return 'every 6 hours';
  }
  if (minutes === 1440) {
    return 'every day';
  }
  if (minutes === 10080) {
    return 'every week';
  }
  if (minutes < 120) {
    return `every ${minutes} minutes`;
  } else if (minutes < 2880) {
    let hours = Math.round(minutes / 60);
    return `every ${hours} hours`;
  } else {
    let days = Math.round(minutes / 1440);
    return `every ${days} days`;
  }
}

function setUpdatePeriodOptions(selectedMinutesValue) {
  if (!addBlocklistUpdatePeriod) {
    return;
  }
  const selectedMinutes = normalizeUpdatePeriodMinutes(selectedMinutesValue);
  addBlocklistUpdatePeriod.innerHTML = '';

  const presetMinutes = new Set(UPDATE_PERIOD_PRESET_OPTIONS.map((option) => option.minutes));
  UPDATE_PERIOD_PRESET_OPTIONS.forEach((option) => {
    const optionEl = document.createElement('option');
    optionEl.value = String(option.minutes);
    optionEl.textContent = option.label;
    addBlocklistUpdatePeriod.appendChild(optionEl);
  });
  if (!presetMinutes.has(selectedMinutes)) {
    const extraOption = document.createElement('option');
    extraOption.value = String(selectedMinutes);
    extraOption.textContent = formatUpdatePeriodForDisplay(selectedMinutes);
    addBlocklistUpdatePeriod.appendChild(extraOption);
  }
  addBlocklistUpdatePeriod.value = String(selectedMinutes);
}

function animateBlocklistReflow(table, mutateLayout) {
  if (!table || typeof mutateLayout !== 'function') {
    return;
  }

  const cards = Array.from(table.querySelectorAll('.blocklist-card'));
  const firstPositions = new Map();
  cards.forEach((card) => {
    firstPositions.set(card, card.getBoundingClientRect().top);
  });

  mutateLayout();

  cards.forEach((card) => {
    const firstTop = firstPositions.get(card);
    if (firstTop === undefined) {
      return;
    }

    const lastTop = card.getBoundingClientRect().top;
    const deltaY = firstTop - lastTop;
    if (Math.abs(deltaY) < 0.5) {
      return;
    }

    card.style.transition = 'none';
    card.style.transform = `translateY(${deltaY}px)`;
    card.getBoundingClientRect();

    requestAnimationFrame(() => {
      card.style.transition = 'transform 180ms ease';
      card.style.transform = '';
    });

    card.addEventListener('transitionend', () => {
      card.style.transition = '';
      card.style.transform = '';
    }, { once: true });
  });
}

function renderBlocklist(blocklist) {
  const container = document.createElement('div');
  container.className = 'blocklist-card';
  container.dataset.blocklistId = String(blocklist.id);
  if (blocklist.id === getSelectedBlocklistId()) {
    container.classList.add('is-selected');
  }

  const headlineRow = document.createElement('div');
  headlineRow.className = 'blocklist-card-headline';

  const name = document.createElement('div');
  name.className = 'blocklist-name';
  name.textContent = blocklist.name;
  headlineRow.appendChild(name);

  const actions = document.createElement('div');
  actions.className = 'blocklist-card-actions';

  if (blocklist.id >= 0) {
    const editButton = document.createElement('button');
    editButton.type = 'button';
    editButton.className = 'blocklist-card-action';
    editButton.title = 'Edit blocklist';
    editButton.textContent = '✏️';
    editButton.addEventListener('click', (event) => {
      event.stopPropagation();
      openBlocklistModal(blocklist);
    });
    actions.appendChild(editButton);

    const deleteButton = document.createElement('button');
    deleteButton.type = 'button';
    deleteButton.className = 'blocklist-card-action';
    deleteButton.title = 'Delete blocklist';
    deleteButton.textContent = '🗑️';
    deleteButton.addEventListener('click', (event) => {
      event.stopPropagation();
      if (!window.confirm(`Delete blocklist "${blocklist.name}"?`)) {
        return;
      }
      if (window.app && typeof window.app.sendAction === 'function') {
        window.app.sendAction('deleteBlocklist', { blocklistId: blocklist.id });
      }
    });
    actions.appendChild(deleteButton);
  }

  headlineRow.appendChild(actions);
  container.appendChild(headlineRow);

  const description = document.createElement('div');
  description.className = 'blocklist-description';
  description.textContent = blocklist.description || '';
  container.appendChild(description);

  if (blocklist.updateFromUrl !== null && blocklist.updateFromUrl !== '') {
    const sourceRow = document.createElement('div');
    sourceRow.className = 'blocklist-source-row';

    const title = document.createElement('div');
    title.className = 'blocklist-source-label';
    title.textContent = 'Update From:';
    sourceRow.appendChild(title);

    const url = document.createElement('div');
    url.className = 'blocklist-source-url';
    url.textContent = blocklist.updateFromUrl;
    sourceRow.appendChild(url);

    container.appendChild(sourceRow);

    const updatePeriodRow = document.createElement('div');
    updatePeriodRow.className = 'blocklist-source-row';

    const updatePeriodLabel = document.createElement('div');
    updatePeriodLabel.className = 'blocklist-source-label';
    updatePeriodLabel.textContent = 'Update:';
    updatePeriodRow.appendChild(updatePeriodLabel);

    const updatePeriodValue = document.createElement('div');
    updatePeriodValue.className = 'blocklist-update-row';
    updatePeriodValue.textContent = formatUpdatePeriodForDisplay(blocklist.updatePeriodMinutes);
    updatePeriodRow.appendChild(updatePeriodValue);

    container.appendChild(updatePeriodRow);

    const lastUpdateSeconds = Number(blocklist.lastUpdate ?? 0);
    const lastSuccessfulSeconds = Number(blocklist.lastSuccessfulUpdate ?? 0);
    const hasLastUpdate = Number.isFinite(lastUpdateSeconds) && lastUpdateSeconds > 0;
    const hasLastSuccessful = Number.isFinite(lastSuccessfulSeconds) && lastSuccessfulSeconds > 0;
    const lastUpdateText = hasLastUpdate ? formatLocalTime(lastUpdateSeconds) : '';
    const lastSuccessfulText = hasLastSuccessful ? formatLocalTime(lastSuccessfulSeconds) : '';
    const lastUpdateError = blocklist.lastUpdateError;
    const updateMetaRow = document.createElement('div');
    updateMetaRow.className = 'blocklist-update-meta-row';

    const updateStatus = document.createElement('div');
    updateStatus.className = 'blocklist-update-row';
    if (lastUpdateError !== null && lastUpdateError !== undefined && lastUpdateError !== '') {
      updateStatus.classList.add('blocklist-update-row-error');
      updateStatus.textContent = `Last update failed ${lastUpdateText ? `(${lastUpdateText})` : ''}: ${lastUpdateError}`;
    } else if (hasLastUpdate) {
      updateStatus.textContent = `Last update: ${lastUpdateText}`;
    } else {
      updateStatus.textContent = 'Last update: never';
    }
    updateMetaRow.appendChild(updateStatus);

    const enabledLabel = document.createElement('label');
    enabledLabel.className = 'blocklist-modal-checkbox-label';
    enabledLabel.title = 'enabled';
    const enabledInput = document.createElement('input');
    enabledInput.type = 'checkbox';
    enabledInput.className = 'blocklist-entry-checkbox';
    enabledInput.checked = blocklist.disabled !== true;
    enabledInput.disabled = blocklist.id < 0;
    enabledInput.addEventListener('click', (event) => {
      event.stopPropagation();
    });
    enabledInput.addEventListener('change', (event) => {
      event.stopPropagation();
      if (blocklist.id < 0) {
        return;
      }
      blocklist.disabled = !enabledInput.checked;
      const knownBlocklist = blocklistsById.get(blocklist.id);
      if (knownBlocklist) {
        knownBlocklist.disabled = blocklist.disabled;
      }
      if (window.app && typeof window.app.sendAction === 'function') {
        window.app.sendAction('setBlocklistEnabled', {
          blocklistId: blocklist.id,
          enabled: enabledInput.checked,
        });
      }
    });
    enabledLabel.appendChild(enabledInput);
    enabledLabel.appendChild(document.createTextNode(' enabled'));
    updateMetaRow.appendChild(enabledLabel);
    container.appendChild(updateMetaRow);

    if (
      (lastUpdateError !== null && lastUpdateError !== undefined && lastUpdateError !== '')
      && hasLastSuccessful
    ) {
      const successRow = document.createElement('div');
      successRow.className = 'blocklist-update-row';
      successRow.textContent = `Last successful update: ${lastSuccessfulText}`;
      container.appendChild(successRow);
    }
  }

  container.addEventListener('click', () => {
    const previousSelection = getSelectedBlocklistId();
    const nextSelection = blocklist.id;
    if (previousSelection !== nextSelection && previousSelection === getUserBlocklistId()) {
      clearSelectedUserEntries();
    }
    const table = document.getElementById('blocklists');
    animateBlocklistReflow(table, () => {
      setSelectedBlocklistId(nextSelection);

      if (window.app && typeof window.app.sendAction === 'function') {
        window.app.sendAction('selectBlocklist', { id: nextSelection });
      }

      table.querySelectorAll('.blocklist-card').forEach((card) => {
        card.classList.remove('is-selected');
      });
      container.classList.add('is-selected');
    });

    renderBlocklistDetails();
  });

  return container;
}

function parseBlocklistIdFromCard(card) {
  if (!card) {
    return null;
  }
  const id = Number(card.dataset.blocklistId);
  return Number.isFinite(id) ? id : null;
}

function navigateBlocklistsSelection(delta) {
  if (delta !== 1 && delta !== -1) {
    return false;
  }
  const list = document.getElementById('blocklists');
  if (!list) {
    return false;
  }
  const cards = Array.from(list.querySelectorAll('.blocklist-card'));
  if (cards.length === 0) {
    return false;
  }

  const selectedId = getSelectedBlocklistId();
  let currentIndex = cards.findIndex((card) => card.classList.contains('is-selected'));
  if (currentIndex < 0) {
    currentIndex = cards.findIndex((card) => parseBlocklistIdFromCard(card) === selectedId);
  }
  if (currentIndex < 0) {
    currentIndex = delta > 0 ? -1 : cards.length;
  }

  const nextIndex = Math.max(0, Math.min(cards.length - 1, currentIndex + delta));
  const nextCard = cards[nextIndex];
  if (!nextCard) {
    return false;
  }
  nextCard.click();
  nextCard.scrollIntoView({ block: 'nearest' });
  return true;
}

window.navigateBlocklistsSelection = navigateBlocklistsSelection;

function setDetailsEmptyState(container, text) {
  container.innerHTML = '';
  const empty = document.createElement('div');
  empty.className = 'blocklist-details-empty';
  empty.textContent = text;
  container.appendChild(empty);
}

function renderEntryRow(entry, index) {
  const row = document.createElement('div');
  row.className = 'blocklist-entry-row';

  if (
    highlightedEntry
    && highlightedEntry.blocklistId === getSelectedBlocklistId()
    && entryMatchesEntryRef(entry, highlightedEntry)
  ) {
    row.classList.add('is-highlighted');
  }

  const checkbox = document.createElement('input');
  checkbox.type = 'checkbox';
  checkbox.className = 'blocklist-entry-checkbox';
  checkbox.checked = !entry.isDisabled;
  checkbox.addEventListener('click', (event) => {
    event.stopPropagation();
  });
  checkbox.addEventListener('change', () => {
    if (!window.app || typeof window.app.sendAction !== 'function') {
      return;
    }
    const value = entry.value;
    if (!value) {
      return;
    }
    const entryType = entry.entryType;
    window.app.sendAction('toggleBlocklistEntryEnabled', { entryType, value });
  });
  row.appendChild(checkbox);

  const title = document.createElement('div');
  title.className = 'blocklist-entry-title';
  title.textContent = formatEntryTitle(entry);
  row.appendChild(title);

  if (isUserBlocklistSelected()) {
    const key = entrySelectionKey(entry.entryType, entry.value);
    if (selectedUserEntries.has(key)) {
      row.classList.add('is-selected');
    }
    row.addEventListener('click', (event) => {
      const details = ensureBlocklistDetails(getSelectedBlocklistId());

      if (event.shiftKey && userSelectionAnchorIndex !== null) {
        const start = Math.min(userSelectionAnchorIndex, index);
        const end = Math.max(userSelectionAnchorIndex, index);
        selectedUserEntries.clear();
        for (let i = start; i <= end; i += 1) {
          const rangeEntry = details.entries.get(i);
          if (!rangeEntry || !rangeEntry.value) {
            continue;
          }
          const rangeKey = entrySelectionKey(rangeEntry.entryType, rangeEntry.value);
          selectedUserEntries.set(rangeKey, {
            entryType: rangeEntry.entryType,
            value: rangeEntry.value,
          });
        }
        userSelectionAnchorIndex = index;
      } else if (event.metaKey || event.ctrlKey) {
        if (selectedUserEntries.has(key)) {
          selectedUserEntries.delete(key);
        } else {
          selectedUserEntries.set(key, { entryType: entry.entryType, value: entry.value });
        }
        userSelectionAnchorIndex = index;
      } else {
        selectedUserEntries.clear();
        selectedUserEntries.set(key, { entryType: entry.entryType, value: entry.value });
        userSelectionAnchorIndex = index;
      }

      refreshVirtualList();
      setupBlocklistDetailsHeaderAddButton();
    });
  }

  return row;
}

function renderPlaceholderRow() {
  const row = document.createElement('div');
  row.className = 'blocklist-entry-row is-placeholder';

  const spacer = document.createElement('div');
  spacer.className = 'blocklist-entry-checkbox';
  row.appendChild(spacer);

  const label = document.createElement('div');
  label.className = 'blocklist-entry-title';
  label.textContent = 'Loading…';
  row.appendChild(label);

  return row;
}

function requestMissingRanges(ctx, startIndex, endIndex) {
  const details = ensureBlocklistDetails(ctx.blocklistId);
  const loaded = details.entries;

  if (details.pending.size >= VIRTUAL_MAX_IN_FLIGHT) {
    return;
  }

  let firstMissing = null;
  for (let i = startIndex; i <= endIndex; i += 1) {
    if (!loaded.has(i)) {
      firstMissing = i;
      break;
    }
  }
  if (firstMissing === null) {
    return;
  }

  const rangeStart = firstMissing;
  let rangeEnd = firstMissing;
  while (
    rangeEnd + 1 <= endIndex
    && !loaded.has(rangeEnd + 1)
    && (rangeEnd - rangeStart + 1) < VIRTUAL_PAGE_SIZE
  ) {
    rangeEnd += 1;
  }

  const key = `${rangeStart}:${rangeEnd}`;
  if (details.pending.has(key)) {
    return;
  }
  if (!window.app || typeof window.app.sendAction !== 'function') {
    return;
  }

  details.pending.add(key);
  window.app.sendAction('loadBlocklistEntries', {
    blocklistId: ctx.blocklistId,
    start: rangeStart,
    limit: rangeEnd - rangeStart + 1,
  });
}

function clearPendingForRange(details, start, end) {
  const toDelete = [];
  details.pending.forEach((key) => {
    const parts = key.split(':');
    if (parts.length !== 2) {
      return;
    }
    const pendingStart = Number(parts[0]);
    const pendingEnd = Number(parts[1]);
    if (Number.isNaN(pendingStart) || Number.isNaN(pendingEnd)) {
      return;
    }
    // Drop any pending request that overlaps this returned chunk.
    if (pendingStart <= end && pendingEnd >= start) {
      toDelete.push(key);
    }
  });
  toDelete.forEach((key) => details.pending.delete(key));
}

// Render only the rows currently in (or near) the visible viewport.
// Top and bottom spacer divs fill the space occupied by off-screen rows so
// the scrollbar thumb reflects the full list length. Missing entries trigger
// a backend fetch and are shown as placeholders until the data arrives.
function renderVirtualListRows(ctx) {
  const details = ensureBlocklistDetails(ctx.blocklistId);
  const total = details.totalEntries || 0;

  if (total === 0) {
    ctx.rowsContainer.innerHTML = '';
    ctx.topSpacer.style.height = '0px';
    ctx.bottomSpacer.style.height = '0px';
    const empty = document.createElement('div');
    empty.className = 'blocklist-entry-empty';
    empty.textContent = getEmptyEntriesText();
    ctx.rowsContainer.appendChild(empty);
    return;
  }

  const scrollTop = ctx.listEl.scrollTop;
  const visibleRows = Math.max(1, Math.ceil(ctx.listEl.clientHeight / VIRTUAL_ROW_HEIGHT));

  const startIndex = Math.max(0, Math.floor(scrollTop / VIRTUAL_ROW_HEIGHT) - VIRTUAL_OVERSCAN);
  const endIndex = Math.min(total - 1, startIndex + visibleRows + (VIRTUAL_OVERSCAN * 2));

  requestMissingRanges(ctx, startIndex, endIndex);

  ctx.topSpacer.style.height = `${startIndex * VIRTUAL_ROW_HEIGHT}px`;
  ctx.bottomSpacer.style.height = `${Math.max(0, (total - endIndex - 1) * VIRTUAL_ROW_HEIGHT)}px`;

  const loaded = details.entries;
  ctx.rowsContainer.innerHTML = '';

  for (let idx = startIndex; idx <= endIndex; idx += 1) {
    const entry = loaded.get(idx);
    if (entry) {
      ctx.rowsContainer.appendChild(renderEntryRow(entry, idx));
    } else {
      ctx.rowsContainer.appendChild(renderPlaceholderRow());
    }
  }
}

function refreshVirtualList() {
  if (activeVirtualList) {
    renderVirtualListRows(activeVirtualList);
  }
}

function renderBlocklistDetails() {
  const detailsContainer = getBlocklistsDetailsContainer();
  if (!detailsContainer) {
    return;
  }

  const selectedId = getSelectedBlocklistId();
  const details = ensureBlocklistDetails(selectedId);
  setupBlocklistDetailsHeaderAddButton();

  if (activeDetailsBlocklistId === selectedId && activeVirtualList) {
    const count = detailsContainer.querySelector('.blocklist-detail-count');
    if (count) {
      count.textContent = `${(details.totalEntries || 0).toLocaleString()} entries`;
    }
    refreshVirtualList();
    return;
  }

  activeDetailsBlocklistId = selectedId;

  const wrapper = document.createElement('div');
  wrapper.className = 'blocklist-details';

  const headlineRow = document.createElement('div');
  headlineRow.className = 'blocklist-detail-headline-row';

  const headline = document.createElement('div');
  headline.className = 'blocklist-detail-headline';
  headline.textContent = 'Blocked entries';
  headlineRow.appendChild(headline);

  const count = document.createElement('div');
  count.className = 'blocklist-detail-count';
  count.textContent = `${(details.totalEntries || 0).toLocaleString()} entries`;
  headlineRow.appendChild(count);
  setupBlocklistDetailsHeaderAddButton();

  wrapper.appendChild(headlineRow);

  const listEl = document.createElement('div');
  listEl.className = 'blocklist-entry-list';

  if ((details.totalEntries || 0) === 0) {
    const empty = document.createElement('div');
    empty.className = 'blocklist-entry-empty';
    empty.textContent = getEmptyEntriesText();
    listEl.appendChild(empty);
    wrapper.appendChild(listEl);
    detailsContainer.innerHTML = '';
    detailsContainer.appendChild(wrapper);
    activeVirtualList = null;
    return;
  }

  const topSpacer = document.createElement('div');
  const rowsContainer = document.createElement('div');
  const bottomSpacer = document.createElement('div');

  listEl.appendChild(topSpacer);
  listEl.appendChild(rowsContainer);
  listEl.appendChild(bottomSpacer);

  activeVirtualList = {
    blocklistId: selectedId,
    listEl,
    topSpacer,
    rowsContainer,
    bottomSpacer,
  };

  listEl.addEventListener('scroll', () => {
    renderVirtualListRows(activeVirtualList);
  });

  wrapper.appendChild(listEl);

  detailsContainer.innerHTML = '';
  detailsContainer.appendChild(wrapper);

  renderVirtualListRows(activeVirtualList);
}

function handleSetBlocklists(msg) {
  setupBlocklistHeaderAddButton();
  setupBlocklistDetailsHeaderAddButton();

  const blocklists = msg.blocklists;
  blocklistsById.clear();
  for (const blocklist of blocklists) {
    blocklistsById.set(blocklist.id, blocklist);
  }

  const table = document.getElementById('blocklists');
  table.classList.add('blocklists-list');
  table.innerHTML = '';

  const selected = getSelectedBlocklistId();
  if (!blocklists.some((b) => b.id === selected)) {
    setSelectedBlocklistId(getUserBlocklistId());
  }

  for (const blocklist of blocklists) {
    table.appendChild(renderBlocklist(blocklist));
  }
  if (window.app && typeof window.app.sendAction === 'function') {
    window.app.sendAction('selectBlocklist', { id: getSelectedBlocklistId() });
  }

  renderBlocklistDetails();
  tryCompletePendingLocate();
}

function handleSetBlocklistDetails(msg) {
  const blocklistId = msg.blocklistId;
  if (blocklistId === null || blocklistId === undefined) {
    return;
  }

  const details = ensureBlocklistDetails(blocklistId);
  details.totalEntries = msg.totalEntries || 0;
  // Invalidate all cached rows because blocklist contents may have changed.
  details.entries.clear();
  details.pending.clear();
  if (blocklistId === getUserBlocklistId()) {
    clearSelectedUserEntries();
    setupBlocklistDetailsHeaderAddButton();
  }

  if (blocklistId === getSelectedBlocklistId()) {
    renderBlocklistDetails();
    tryCompletePendingLocate();
  }
}

function handleSetBlocklistEntries(msg) {
  const blocklistId = msg.blocklistId;
  const start = msg.start;
  const entries = msg.entries;

  if (blocklistId === null || blocklistId === undefined || !Array.isArray(entries)) {
    return;
  }

  const details = ensureBlocklistDetails(blocklistId);
  for (let i = 0; i < entries.length; i += 1) {
    details.entries.set(start + i, entries[i]);
  }

  clearPendingForRange(details, start, start + entries.length - 1);

  if (blocklistId === getSelectedBlocklistId()) {
    refreshVirtualList();
    tryCompletePendingLocate();
  }
}

setupBlocklistHeaderAddButton();
setupBlocklistDetailsHeaderAddButton();
