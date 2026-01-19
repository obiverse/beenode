import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import Card from '../components/Card.jsx';
import { scrollApi } from '../api/scrollApi.js';

const FAVORITES_KEY = 'beenode.scrolls.favorites';
const RECENT_KEY = 'beenode.scrolls.recent';
const BREATH_SEQUENCE = [
  { label: 'Inhale', seconds: 4 },
  { label: 'Hold', seconds: 2 },
  { label: 'Exhale', seconds: 6 }
];
const TAO_PROMPTS = [
  { title: 'Empty Vessel', text: 'Release clutter. Let the scroll show what remains.' },
  { title: 'Wu Wei', text: 'Act by not forcing. Let the next step arrive.' },
  { title: 'Still Water', text: 'Look without chasing. The signal settles on its own.' },
  { title: 'Returning', text: 'Close the loop. Save, then walk away.' }
];

const ensureLeadingSlash = (value) => {
  const trimmed = (value || '').trim();
  if (!trimmed) return '/';
  return trimmed.startsWith('/') ? trimmed : `/${trimmed}`;
};

const normalizePrefix = (value) => {
  let next = ensureLeadingSlash(value);
  if (next.length > 1 && !next.endsWith('/')) next = `${next}/`;
  return next;
};

const normalizePath = (value) => {
  let next = ensureLeadingSlash(value);
  if (next.length > 1 && next.endsWith('/')) next = next.slice(0, -1);
  return next;
};

const parentPrefix = (value) => {
  const normalized = normalizePrefix(value);
  if (normalized === '/') return '/';
  const parts = normalized.split('/').filter(Boolean);
  parts.pop();
  if (parts.length === 0) return '/';
  return `/${parts.join('/')}/`;
};

const buildEntries = (paths, prefix) => {
  const normalized = normalizePrefix(prefix);
  const entries = new Map();
  paths.forEach((path) => {
    if (!path || !path.startsWith(normalized)) return;
    const remainder = path.slice(normalized.length);
    if (!remainder) return;
    const [segment, ...rest] = remainder.split('/');
    if (!segment) return;
    const isFolder = rest.length > 0;
    const entryPath = isFolder ? `${normalized}${segment}/` : `${normalized}${segment}`;
    const existing = entries.get(entryPath);
    if (existing) {
      if (isFolder) existing.count += 1;
      return;
    }
    entries.set(entryPath, {
      name: segment,
      path: entryPath,
      type: isFolder ? 'folder' : 'scroll',
      count: isFolder ? 1 : 0
    });
  });
  return [...entries.values()].sort((a, b) => {
    if (a.type !== b.type) return a.type === 'folder' ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
};

const looksLikeScroll = (value) =>
  value && typeof value === 'object' && ('data' in value || 'metadata' in value || 'key' in value);

const unwrapScroll = (payload) => {
  if (looksLikeScroll(payload)) return payload;
  if (payload && looksLikeScroll(payload.data)) return payload.data;
  return payload;
};

const extractScrollPayload = (payload) => {
  const scroll = unwrapScroll(payload);
  const content = scroll && 'data' in scroll ? scroll.data : scroll;
  return {
    content,
    metadata: scroll && scroll.metadata ? scroll.metadata : null,
    key: scroll && scroll.key ? scroll.key : null,
    type: scroll && scroll.type ? scroll.type : null
  };
};

const loadList = (key) => {
  if (typeof window === 'undefined') return [];
  try {
    const stored = JSON.parse(localStorage.getItem(key) || '[]');
    return Array.isArray(stored) ? stored : [];
  } catch (error) {
    return [];
  }
};

const saveList = (key, value) => {
  if (typeof window === 'undefined') return;
  localStorage.setItem(key, JSON.stringify(value));
};

export default function Scrolls() {
  const [prefixInput, setPrefixInput] = useState('/');
  const [prefix, setPrefix] = useState('/');
  const [listPaths, setListPaths] = useState([]);
  const [listLoading, setListLoading] = useState(false);
  const [listError, setListError] = useState('');
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedPath, setSelectedPath] = useState('');
  const [tabs, setTabs] = useState([]);
  const [activePath, setActivePath] = useState('');
  const [editorText, setEditorText] = useState('');
  const [editorError, setEditorError] = useState('');
  const [statusMessage, setStatusMessage] = useState('Ready to explore.');
  const [newPath, setNewPath] = useState('/notes/new-scroll');
  const [saveAsPath, setSaveAsPath] = useState('');
  const [favorites, setFavorites] = useState(() => loadList(FAVORITES_KEY));
  const [recent, setRecent] = useState(() => loadList(RECENT_KEY));
  const [focusMode, setFocusMode] = useState(false);
  const [promptIndex, setPromptIndex] = useState(0);
  const [breathState, setBreathState] = useState({
    phase: 0,
    remaining: BREATH_SEQUENCE[0].seconds
  });
  const tabsRef = useRef(tabs);

  useEffect(() => {
    tabsRef.current = tabs;
  }, [tabs]);

  useEffect(() => {
    saveList(FAVORITES_KEY, favorites);
  }, [favorites]);

  useEffect(() => {
    saveList(RECENT_KEY, recent);
  }, [recent]);

  useEffect(() => {
    setPrefixInput(prefix);
  }, [prefix]);

  useEffect(() => {
    const timer = setInterval(() => {
      setBreathState((current) => {
        if (current.remaining > 1) {
          return { ...current, remaining: current.remaining - 1 };
        }
        const nextPhase = (current.phase + 1) % BREATH_SEQUENCE.length;
        return {
          phase: nextPhase,
          remaining: BREATH_SEQUENCE[nextPhase].seconds
        };
      });
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const entries = useMemo(() => buildEntries(listPaths, prefix), [listPaths, prefix]);
  const filteredEntries = useMemo(() => {
    const term = searchTerm.trim().toLowerCase();
    if (!term) return entries;
    return entries.filter((entry) => entry.name.toLowerCase().includes(term));
  }, [entries, searchTerm]);

  const breadcrumbs = useMemo(() => {
    const normalized = normalizePrefix(prefix);
    const parts = normalized.split('/').filter(Boolean);
    const crumbs = [{ label: 'root', path: '/' }];
    let current = '/';
    parts.forEach((part) => {
      current = `${current}${part}/`;
      crumbs.push({ label: part, path: current });
    });
    return crumbs;
  }, [prefix]);

  const activeTab = useMemo(
    () => tabs.find((tab) => tab.path === activePath) || null,
    [tabs, activePath]
  );

  useEffect(() => {
    if (!activeTab) {
      setEditorText('');
      setEditorError('');
      setSaveAsPath('');
      return;
    }
    setEditorText(activeTab.contentText || '');
    setEditorError(activeTab.error || '');
    setSaveAsPath(activeTab.path);
  }, [activeTab]);

  const fetchList = useCallback(async () => {
    const normalized = normalizePrefix(prefixInput);
    setListLoading(true);
    setListError('');
    const response = await scrollApi.listScrolls(normalized);
    if (response.ok) {
      const paths = response.data?.paths || response.data?.data?.paths || [];
      setListPaths(paths);
      setPrefix(normalized);
      setStatusMessage(`Listed ${paths.length} scrolls under ${normalized}`);
    } else {
      setListError(response.data?.error || 'Failed to list scrolls.');
      setStatusMessage('List failed.');
    }
    setListLoading(false);
  }, [prefixInput]);

  useEffect(() => {
    fetchList();
  }, [fetchList]);

  const rememberRecent = useCallback((path) => {
    setRecent((prev) => {
      const normalized = normalizePath(path);
      const next = [normalized, ...prev.filter((item) => item !== normalized)];
      return next.slice(0, 8);
    });
  }, []);

  const openTab = useCallback(async (path, { refresh } = { refresh: false }) => {
    const normalized = normalizePath(path);
    const existing = tabsRef.current.find((tab) => tab.path === normalized);
    if (existing && !refresh) {
      setActivePath(normalized);
      setStatusMessage(`Focused ${normalized}`);
      return;
    }
    setStatusMessage(`Loading ${normalized}...`);
    const response = await scrollApi.scrollGet(normalized);
    if (response.ok) {
      const { content, metadata, key, type } = extractScrollPayload(response.data);
      const contentText = JSON.stringify(content ?? {}, null, 2);
      setTabs((prev) => {
        const next = prev.filter((tab) => tab.path !== normalized);
        next.push({
          path: normalized,
          key,
          type,
          metadata,
          content,
          contentText,
          dirty: false,
          error: ''
        });
        return next;
      });
      setActivePath(normalized);
      setEditorText(contentText);
      setEditorError('');
      setStatusMessage(`Loaded ${normalized}`);
      rememberRecent(normalized);
      return;
    }
    const errorMessage = response.data?.error || 'Failed to load scroll.';
    setTabs((prev) => {
      if (prev.some((tab) => tab.path === normalized)) return prev;
      return [
        ...prev,
        { path: normalized, contentText: '', dirty: false, error: errorMessage }
      ];
    });
    setActivePath(normalized);
    setEditorText('');
    setEditorError(errorMessage);
    setStatusMessage(errorMessage);
  }, [rememberRecent]);

  const createNewTab = useCallback(() => {
    const normalized = normalizePath(newPath);
    const existing = tabsRef.current.find((tab) => tab.path === normalized);
    if (existing) {
      setActivePath(normalized);
      setStatusMessage(`Focused ${normalized}`);
      return;
    }
    const template = {
      title: 'Untitled Scroll',
      body: '',
      created_at: new Date().toISOString()
    };
    const contentText = JSON.stringify(template, null, 2);
    setTabs((prev) => {
      if (prev.some((tab) => tab.path === normalized)) return prev;
      return [
        ...prev,
        {
          path: normalized,
          key: null,
          type: null,
          metadata: null,
          content: template,
          contentText,
          dirty: true,
          error: ''
        }
      ];
    });
    setActivePath(normalized);
    setEditorText(contentText);
    setEditorError('');
    setStatusMessage(`Drafted ${normalized}`);
  }, [newPath]);

  const closeTab = useCallback((path) => {
    setTabs((prev) => {
      const next = prev.filter((tab) => tab.path !== path);
      if (activePath === path) {
        setActivePath(next[next.length - 1]?.path || '');
      }
      return next;
    });
    setStatusMessage(`Closed ${path}`);
  }, [activePath]);

  const updateActiveTab = useCallback((patch) => {
    if (!activePath) return;
    setTabs((prev) =>
      prev.map((tab) => (tab.path === activePath ? { ...tab, ...patch } : tab))
    );
  }, [activePath]);

  const handleEditorChange = (event) => {
    const value = event.target.value;
    setEditorText(value);
    setEditorError('');
    updateActiveTab({ contentText: value, dirty: true });
  };

  const formatEditor = () => {
    if (!editorText.trim()) return;
    try {
      const parsed = JSON.parse(editorText);
      const formatted = JSON.stringify(parsed, null, 2);
      setEditorText(formatted);
      updateActiveTab({ contentText: formatted, dirty: true });
      setEditorError('');
      setStatusMessage('Formatted JSON.');
    } catch (error) {
      setEditorError(`JSON error: ${error.message}`);
      setStatusMessage('Format failed.');
    }
  };

  const saveActive = useCallback(async () => {
    if (!activePath) return;
    try {
      const target = normalizePath(activePath);
      const parsed = editorText.trim() ? JSON.parse(editorText) : {};
      setStatusMessage(`Saving ${target}...`);
      const response = await scrollApi.scrollWrite(target, parsed);
      if (response.ok) {
        setTabs((prev) => {
          const next = prev.filter((tab) => tab.path !== target);
          next.push({
            path: target,
            content: parsed,
            contentText: editorText,
            metadata: { ...(activeTab?.metadata || {}), version: response.data?.version },
            key: response.data?.key || target,
            type: activeTab?.type || null,
            dirty: false,
            error: ''
          });
          return next;
        });
        setActivePath(target);
        setStatusMessage(`Saved ${target}`);
        rememberRecent(target);
        return;
      }
      setEditorError(response.data?.error || 'Save failed.');
      setStatusMessage('Save failed.');
    } catch (error) {
      setEditorError(`JSON error: ${error.message}`);
      setStatusMessage('Save failed.');
    }
  }, [activePath, activeTab?.metadata, editorText, rememberRecent, updateActiveTab]);

  const saveAs = useCallback(async () => {
    if (!saveAsPath.trim()) return;
    const target = normalizePath(saveAsPath);
    try {
      const parsed = editorText.trim() ? JSON.parse(editorText) : {};
      setStatusMessage(`Saving ${target}...`);
      const response = await scrollApi.scrollWrite(target, parsed);
      if (response.ok) {
        setTabs((prev) => {
          const next = prev.filter((tab) => tab.path !== target);
          next.push({
            path: target,
            content: parsed,
            contentText: editorText,
            metadata: { ...(activeTab?.metadata || {}), version: response.data?.version },
            key: response.data?.key || target,
            type: activeTab?.type || null,
            dirty: false,
            error: ''
          });
          return next;
        });
        setActivePath(target);
        setStatusMessage(`Saved as ${target}`);
        rememberRecent(target);
        return;
      }
      setEditorError(response.data?.error || 'Save as failed.');
      setStatusMessage('Save as failed.');
    } catch (error) {
      setEditorError(`JSON error: ${error.message}`);
      setStatusMessage('Save as failed.');
    }
  }, [activeTab?.metadata, activeTab?.type, editorText, rememberRecent, saveAsPath]);

  const toggleFavorite = (path) => {
    const normalized = normalizePath(path);
    setFavorites((prev) =>
      prev.includes(normalized) ? prev.filter((item) => item !== normalized) : [...prev, normalized]
    );
  };

  const previewText = useMemo(() => {
    if (!editorText.trim()) return 'No content loaded.';
    try {
      const parsed = JSON.parse(editorText);
      return JSON.stringify(parsed, null, 2);
    } catch (error) {
      return editorText;
    }
  }, [editorText]);

  const balanceNote = activeTab?.dirty ? 'Yang rising: changes pending.' : 'Yin calm: scroll is settled.';

  return (
    <div className={`scroll-explorer${focusMode ? ' focus-mode' : ''}`}>
      <div className="scroll-explorer__sidebar scroll-panel">
        <Card>
          <div className="scroll-header">
            <h3>Scroll Explorer</h3>
            <button onClick={fetchList} disabled={listLoading}>
              {listLoading ? 'Listing...' : 'Refresh'}
            </button>
          </div>
          <div className="scroll-crumbs">
            {breadcrumbs.map((crumb) => (
              <button
                key={crumb.path}
                type="button"
                className="scroll-crumb"
                onClick={() => {
                  setPrefixInput(crumb.path);
                  setPrefix(crumb.path);
                }}
              >
                {crumb.label}
              </button>
            ))}
          </div>
          <label>Browse Prefix</label>
          <input
            className="scroll-input"
            value={prefixInput}
            onChange={(event) => setPrefixInput(event.target.value)}
            placeholder="/"
          />
          <div className="scroll-controls">
            <button onClick={() => setPrefixInput(parentPrefix(prefixInput))}>Up</button>
            <button onClick={fetchList}>List</button>
          </div>
          <label>Search</label>
          <input
            className="scroll-input"
            value={searchTerm}
            onChange={(event) => setSearchTerm(event.target.value)}
            placeholder="Filter names"
          />
          {listError && <div className="scroll-error">{listError}</div>}
          {!listLoading && filteredEntries.length === 0 && (
            <div className="utxo-empty">No scrolls found in this prefix.</div>
          )}
          <div className="scroll-list">
            {filteredEntries.map((entry) => (
              <div
                key={entry.path}
                className={`scroll-entry ${entry.type}${selectedPath === entry.path ? ' active' : ''}`}
              >
                <button
                  type="button"
                  className="scroll-entry-btn"
                  onClick={() => {
                    setSelectedPath(entry.path);
                    if (entry.type === 'folder') {
                      setPrefixInput(entry.path);
                      setPrefix(entry.path);
                      return;
                    }
                    openTab(entry.path);
                  }}
                >
                  <span className="scroll-entry-label">{entry.name}</span>
                  <span className="scroll-entry-meta">
                    {entry.type === 'folder' ? `${entry.count} items` : 'scroll'}
                  </span>
                </button>
                {entry.type === 'scroll' && (
                  <button
                    type="button"
                    className={`scroll-fav${favorites.includes(entry.path) ? ' active' : ''}`}
                    onClick={() => toggleFavorite(entry.path)}
                    aria-label="Toggle favorite"
                  >
                    {favorites.includes(entry.path) ? 'Pinned' : 'Pin'}
                  </button>
                )}
              </div>
            ))}
          </div>
        </Card>
        <Card>
          <h3>Open Scrolls</h3>
          {tabs.length === 0 && <div className="utxo-empty">No open scrolls.</div>}
          <div className="scroll-tabs">
            {tabs.map((tab) => (
              <div
                key={tab.path}
                className={`scroll-tab${tab.path === activePath ? ' active' : ''}`}
              >
                <button
                  type="button"
                  className="scroll-tab-button"
                  onClick={() => setActivePath(tab.path)}
                >
                  {tab.path}{tab.dirty ? ' *' : ''}
                </button>
                <button
                  type="button"
                  className="scroll-tab-close"
                  onClick={() => closeTab(tab.path)}
                >
                  x
                </button>
              </div>
            ))}
          </div>
          {favorites.length > 0 && (
            <>
              <h3>Favorites</h3>
              <div className="scroll-list">
                {favorites.map((path) => (
                  <div key={path} className="scroll-entry scroll">
                    <button
                      type="button"
                      className="scroll-entry-btn"
                      onClick={() => openTab(path)}
                    >
                      <span className="scroll-entry-label">{path}</span>
                      <span className="scroll-entry-meta">favorite</span>
                    </button>
                    <button
                      type="button"
                      className="scroll-fav active"
                      onClick={() => toggleFavorite(path)}
                    >
                      Pinned
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}
          {recent.length > 0 && (
            <>
              <h3>Recent</h3>
              <div className="scroll-list">
                {recent.map((path) => (
                  <div key={path} className="scroll-entry scroll">
                    <button
                      type="button"
                      className="scroll-entry-btn"
                      onClick={() => openTab(path)}
                    >
                      <span className="scroll-entry-label">{path}</span>
                      <span className="scroll-entry-meta">recent</span>
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}
        </Card>
      </div>

      <div className="scroll-explorer__main scroll-panel">
        <Card>
          <div className="scroll-header">
            <h3>Compose</h3>
            <div className="scroll-status">{statusMessage}</div>
          </div>
          <label>New Scroll Path</label>
          <input
            className="scroll-input"
            value={newPath}
            onChange={(event) => setNewPath(event.target.value)}
            placeholder="/notes/my-scroll"
          />
          <div className="scroll-controls">
            <button onClick={createNewTab}>New</button>
            <button onClick={() => activePath && openTab(activePath, { refresh: true })} disabled={!activePath}>
              Reload
            </button>
          </div>
          <label>Active Scroll</label>
          <input
            className="scroll-input"
            value={activePath || ''}
            onChange={(event) => setActivePath(event.target.value)}
            placeholder="/path"
          />
          <div className="scroll-controls">
            <button onClick={() => activePath && openTab(activePath)} disabled={!activePath}>
              Open
            </button>
            <button onClick={saveActive} disabled={!activePath}>
              Save
            </button>
            <button onClick={formatEditor} disabled={!activePath}>
              Format
            </button>
          </div>
          <label>Save As</label>
          <input
            className="scroll-input"
            value={saveAsPath}
            onChange={(event) => setSaveAsPath(event.target.value)}
            placeholder="/notes/copy"
          />
          <div className="scroll-controls">
            <button onClick={saveAs} disabled={!saveAsPath}>
              Save As
            </button>
          </div>
          {editorError && <div className="scroll-error">{editorError}</div>}
        </Card>
        <Card>
          <h3>Editor</h3>
          {!activePath && <div className="utxo-empty">Open or create a scroll to edit.</div>}
          <textarea
            className="scroll-editor"
            value={editorText}
            onChange={handleEditorChange}
            placeholder="Write JSON content for the scroll"
          />
        </Card>
      </div>

      <div className="scroll-explorer__inspector scroll-panel">
        <Card>
          <h3>Preview</h3>
          {!activePath && <div className="utxo-empty">No active scroll.</div>}
          {activePath && (
            <>
              <div className="scroll-kicker">{balanceNote}</div>
              {activeTab?.metadata && (
                <div className="scroll-meta">
                  <div>Version: {activeTab.metadata.version ?? 'n/a'}</div>
                  <div>Updated: {activeTab.metadata.updated_at ?? 'n/a'}</div>
                </div>
              )}
              <pre className="scroll-preview">{previewText}</pre>
            </>
          )}
        </Card>
        <Card>
          <h3>Tao Tools</h3>
          <div className="scroll-tool">
            <div className="scroll-tool-label">Breath Cycle</div>
            <div className="scroll-tool-value">
              {BREATH_SEQUENCE[breathState.phase].label} ({breathState.remaining}s)
            </div>
          </div>
          <div className="scroll-tool">
            <div className="scroll-tool-label">{TAO_PROMPTS[promptIndex].title}</div>
            <div className="scroll-tool-value">{TAO_PROMPTS[promptIndex].text}</div>
          </div>
          <div className="scroll-controls">
            <button
              onClick={() => setPromptIndex((index) => (index + 1) % TAO_PROMPTS.length)}
            >
              Next Reflection
            </button>
            <button onClick={() => setFocusMode((prev) => !prev)}>
              {focusMode ? 'Exit Focus' : 'Enter Focus'}
            </button>
          </div>
        </Card>
      </div>
    </div>
  );
}
