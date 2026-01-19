import { memo, useEffect, useRef, useState } from 'react';
import Window from './Window.jsx';
import Home from '../apps/Home.jsx';
import Scrolls from '../apps/Scrolls.jsx';
import Settings from '../apps/Settings.jsx';
import RevealTool from '../apps/RevealTool.jsx';
import SealCreator from '../apps/SealCreator.jsx';
import SealExamples from '../apps/SealExamples.jsx';
import TxForge from '../apps/TxForge.jsx';
import UtxoExplorer from '../apps/UtxoExplorer.jsx';
import Wallet from '../apps/Wallet.jsx';
import { useAuth } from '../contexts/AuthContext.jsx';
import LandingPage from './LandingPage.jsx';

const apps = [
  { id: 'home', label: 'Home', icon: '🏠' },
  { id: 'wallet', label: 'Wallet', icon: '💰' },
  { id: 'utxos', label: 'UTXOs', icon: '📦' },
  { id: 'txforge', label: 'TX Forge', icon: '🔨' },
  { id: 'seal', label: 'Seal', icon: '🔏' },
  { id: 'examples', label: 'Examples', icon: '📚' },
  { id: 'reveal', label: 'Reveal', icon: '🔓' },
  { id: 'scrolls', label: 'Scrolls', icon: '📜' },
  { id: 'settings', label: 'Settings', icon: '⚙️' }
];

const layerDetails = {
  public: { label: 'public', emoji: '🌍' },
  protected: { label: 'protected', emoji: '🔒' },
  admin: { label: 'admin', emoji: '⚙️' }
};

function DesktopShell({ onCheckHealth, onSyncWallet, onGetBalance }) {
  const { state, unlock, unlockAdmin, setProtected, lock } = useAuth();
  const WINDOW_MIN_WIDTH = 400;
  const WINDOW_MIN_HEIGHT = 300;
  const WINDOW_MAX_RATIO = 0.8;
  const WINDOW_CONTENT_PADDING = 32;
  const WINDOW_MARGIN = 20;
  const GOLDEN_ANGLE = Math.PI * (3 - Math.sqrt(5));

  const [openWindows, setOpenWindows] = useState([]);
  const [activeWindowId, setActiveWindowId] = useState(null);
  const [windowData, setWindowData] = useState(() =>
    apps.reduce((acc, app) => {
      acc[app.id] = {
        isOpen: false,
        isMinimized: false,
        isMaximized: false,
        zIndex: 10,
        position: { left: 0, top: 0 },
        size: { width: null, height: null },
        userSized: false,
        userPositioned: false,
        needsLayout: false,
        normalRect: null
      };
      return acc;
    }, {})
  );

  const windowLayerRef = useRef(null);
  const windowRefs = useRef({});
  const windowDataRef = useRef(windowData);
  const openWindowsRef = useRef(openWindows);
  const zIndexCounter = useRef(20);
  const phiRef = useRef(1.618034);
  const dragStateRef = useRef(null);
  const resizeStateRef = useRef(null);

  useEffect(() => {
    windowDataRef.current = windowData;
  }, [windowData]);

  useEffect(() => {
    openWindowsRef.current = openWindows;
  }, [openWindows]);

  useEffect(() => {
    const storedPhi = parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue('--phi')
    );
    if (!Number.isNaN(storedPhi)) {
      phiRef.current = storedPhi;
    }
  }, []);

  const clamp = (value, min, max) => Math.min(Math.max(value, min), max);

  const getBounds = () => {
    const layer = windowLayerRef.current;
    if (!layer) {
      return { width: window.innerWidth, height: window.innerHeight, left: 0, top: 0 };
    }
    const rect = layer.getBoundingClientRect();
    return { width: rect.width, height: rect.height, left: rect.left, top: rect.top };
  };

  const getDefaultWindowSize = () => ({
    width: WINDOW_MIN_WIDTH,
    height: WINDOW_MIN_HEIGHT
  });

  const getMaxWindowSize = (bounds) => ({
    width: Math.max(WINDOW_MIN_WIDTH, Math.round(bounds.width * WINDOW_MAX_RATIO)),
    height: Math.max(WINDOW_MIN_HEIGHT, Math.round(bounds.height * WINDOW_MAX_RATIO))
  });

  const getWindowKey = (appId) => `beenode.window.size.${appId}`;

  const getStoredSize = (appId) => {
    const raw = localStorage.getItem(getWindowKey(appId));
    if (!raw) return null;
    try {
      const parsed = JSON.parse(raw);
      if (!parsed || !parsed.width || !parsed.height) return null;
      return parsed;
    } catch (error) {
      return null;
    }
  };

  const saveWindowSize = (appId, size) => {
    if (!appId || !size?.width || !size?.height) return;
    localStorage.setItem(
      getWindowKey(appId),
      JSON.stringify({ width: Math.round(size.width), height: Math.round(size.height) })
    );
  };

  const computePosition = (appId, width, height, bounds) => {
    const data = windowDataRef.current[appId];
    if (data?.userPositioned) {
      return data.position;
    }
    const centerX = (bounds.width - width) / 2;
    const centerY = (bounds.height - height) / 2;
    const index = Math.max(openWindowsRef.current.indexOf(appId), 0);
    const radius = index === 0 ? 0 : Math.min(140, 24 + index * 18);
    const left = clamp(
      centerX + Math.cos(index * GOLDEN_ANGLE) * radius,
      WINDOW_MARGIN,
      bounds.width - width - WINDOW_MARGIN
    );
    const top = clamp(
      centerY + Math.sin(index * GOLDEN_ANGLE) * radius,
      WINDOW_MARGIN,
      bounds.height - height - WINDOW_MARGIN
    );
    return { left, top };
  };

  const openApp = (appId) => {
    setOpenWindows((prev) => (prev.includes(appId) ? prev : [...prev, appId]));
    setWindowData((prev) => {
      const data = prev[appId];
      if (!data) return prev;
      const nextZ = zIndexCounter.current + 1;
      zIndexCounter.current = nextZ;
      return {
        ...prev,
        [appId]: {
          ...data,
          isOpen: true,
          isMinimized: false,
          zIndex: nextZ,
          needsLayout: !data.userSized || !data.userPositioned || !data.size?.width || !data.size?.height
        }
      };
    });
    setActiveWindowId(appId);
  };

  const focusWindow = (appId) => {
    setWindowData((prev) => {
      const data = prev[appId];
      if (!data || !data.isOpen || data.isMinimized) return prev;
      const nextZ = zIndexCounter.current + 1;
      zIndexCounter.current = nextZ;
      return {
        ...prev,
        [appId]: {
          ...data,
          zIndex: nextZ
        }
      };
    });
    setActiveWindowId(appId);
  };

  const closeWindow = (appId) => {
    setWindowData((prev) => {
      const data = prev[appId];
      if (!data) return prev;
      return {
        ...prev,
        [appId]: {
          ...data,
          isOpen: false,
          isMinimized: false,
          isMaximized: false,
          normalRect: null
        }
      };
    });
    setOpenWindows((prev) => prev.filter((id) => id !== appId));
  };

  const minimizeWindow = (appId) => {
    setWindowData((prev) => {
      const data = prev[appId];
      if (!data || !data.isOpen) return prev;
      return {
        ...prev,
        [appId]: {
          ...data,
          isMinimized: true
        }
      };
    });
  };

  const toggleMaximize = (appId) => {
    setWindowData((prev) => {
      const data = prev[appId];
      if (!data || !data.isOpen) return prev;
      if (data.isMaximized) {
        if (data.normalRect) {
          return {
            ...prev,
            [appId]: {
              ...data,
              isMaximized: false,
              position: { left: data.normalRect.left, top: data.normalRect.top },
              size: { width: data.normalRect.width, height: data.normalRect.height },
              userSized: true,
              userPositioned: true,
              normalRect: null
            }
          };
        }
        return {
          ...prev,
          [appId]: {
            ...data,
            isMaximized: false,
            needsLayout: true,
            normalRect: null
          }
        };
      }
      const bounds = getBounds();
      const margin = WINDOW_MARGIN;
      const size = data.size?.width
        ? data.size
        : { width: WINDOW_MIN_WIDTH, height: WINDOW_MIN_HEIGHT };
      const normalRect = {
        left: data.position?.left ?? margin,
        top: data.position?.top ?? margin,
        width: size.width,
        height: size.height
      };
      return {
        ...prev,
        [appId]: {
          ...data,
          isMaximized: true,
          isMinimized: false,
          position: { left: margin, top: margin },
          size: {
            width: Math.max(bounds.width - margin * 2, WINDOW_MIN_WIDTH),
            height: Math.max(bounds.height - margin * 2, WINDOW_MIN_HEIGHT)
          },
          normalRect
        }
      };
    });
    focusWindow(appId);
  };

  const handleDragStart = (event, appId) => {
    if (event.button !== 0) return;
    const data = windowDataRef.current[appId];
    if (!data || data.isMaximized || data.isMinimized) return;
    const windowEl = windowRefs.current[appId]?.windowEl;
    if (!windowEl) return;
    const rect = windowEl.getBoundingClientRect();
    dragStateRef.current = {
      appId,
      offsetX: event.clientX - rect.left,
      offsetY: event.clientY - rect.top
    };
    setWindowData((prev) => ({
      ...prev,
      [appId]: {
        ...prev[appId],
        userPositioned: true
      }
    }));
    focusWindow(appId);
    event.preventDefault();
  };

  const handleResizeStart = (event, appId, direction) => {
    if (event.button !== 0) return;
    const data = windowDataRef.current[appId];
    if (!data || data.isMaximized || data.isMinimized) return;
    const windowEl = windowRefs.current[appId]?.windowEl;
    if (!windowEl) return;
    resizeStateRef.current = {
      appId,
      direction,
      startX: event.clientX,
      startY: event.clientY,
      startWidth: windowEl.offsetWidth,
      startHeight: windowEl.offsetHeight,
      startLeft: windowEl.offsetLeft,
      startTop: windowEl.offsetTop
    };
    setWindowData((prev) => ({
      ...prev,
      [appId]: {
        ...prev[appId],
        userSized: true,
        userPositioned: true
      }
    }));
    focusWindow(appId);
    event.preventDefault();
  };

  useEffect(() => {
    const handleMouseMove = (event) => {
      if (dragStateRef.current) {
        const { appId, offsetX, offsetY } = dragStateRef.current;
        const data = windowDataRef.current[appId];
        if (!data) return;
        const bounds = getBounds();
        const width = data.size?.width ?? WINDOW_MIN_WIDTH;
        const height = data.size?.height ?? WINDOW_MIN_HEIGHT;
        const left = clamp(
          event.clientX - bounds.left - offsetX,
          WINDOW_MARGIN,
          bounds.width - width - WINDOW_MARGIN
        );
        const top = clamp(
          event.clientY - bounds.top - offsetY,
          WINDOW_MARGIN,
          bounds.height - height - WINDOW_MARGIN
        );
        setWindowData((prev) => ({
          ...prev,
          [appId]: {
            ...prev[appId],
            position: { left, top },
            userPositioned: true
          }
        }));
        return;
      }

      if (!resizeStateRef.current) return;
      const { appId, direction, startX, startY, startWidth, startHeight, startLeft, startTop } =
        resizeStateRef.current;
      const bounds = getBounds();
      const { width: minWidth, height: minHeight } = getDefaultWindowSize(bounds);
      const { width: maxWidth, height: maxHeight } = getMaxWindowSize(bounds);
      let width = startWidth;
      let height = startHeight;
      let left = startLeft;
      let top = startTop;
      const dx = event.clientX - startX;
      const dy = event.clientY - startY;

      if (direction.includes('e')) {
        width = clamp(startWidth + dx, minWidth, maxWidth);
      }
      if (direction.includes('s')) {
        height = clamp(startHeight + dy, minHeight, maxHeight);
      }
      if (direction.includes('w')) {
        width = clamp(startWidth - dx, minWidth, maxWidth);
        left = clamp(startLeft + dx, WINDOW_MARGIN, bounds.width - width - WINDOW_MARGIN);
      }
      if (direction.includes('n')) {
        height = clamp(startHeight - dy, minHeight, maxHeight);
        top = clamp(startTop + dy, WINDOW_MARGIN, bounds.height - height - WINDOW_MARGIN);
      }

      setWindowData((prev) => ({
        ...prev,
        [appId]: {
          ...prev[appId],
          size: { width, height },
          position: { left, top },
          userSized: true,
          userPositioned: true
        }
      }));
    };

    const handleMouseUp = () => {
      if (resizeStateRef.current) {
        const { appId } = resizeStateRef.current;
        const data = windowDataRef.current[appId];
        if (data && !data.isMaximized) {
          saveWindowSize(appId, data.size);
        }
      }
      dragStateRef.current = null;
      resizeStateRef.current = null;
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

  useEffect(() => {
    const pending = openWindows.filter((id) => windowData[id]?.isOpen && windowData[id]?.needsLayout);
    if (pending.length === 0) return undefined;
    const frame = requestAnimationFrame(() => {
      setWindowData((prev) => {
        const next = { ...prev };
        pending.forEach((appId) => {
          const data = prev[appId];
          if (!data || !data.isOpen || !data.needsLayout) return;
          const windowEl = windowRefs.current[appId]?.windowEl;
          const contentEl = windowRefs.current[appId]?.contentEl;
          if (!windowEl || !contentEl) return;
          const bounds = getBounds();
          const stored = getStoredSize(appId);
          const { width: minWidth, height: minHeight } = getDefaultWindowSize(bounds);
          const { width: maxWidth, height: maxHeight } = getMaxWindowSize(bounds);
          let width = stored?.width;
          let height = stored?.height;
          let userSized = data.userSized;

          if (width && height) {
            width = clamp(width, minWidth, maxWidth);
            height = clamp(height, minHeight, maxHeight);
            userSized = true;
          } else {
            const titleEl = windowEl.querySelector('.window-title');
            const titleHeight = titleEl ? titleEl.offsetHeight : 0;
            width = clamp(
              Math.max(contentEl.scrollWidth + WINDOW_CONTENT_PADDING, minWidth),
              minWidth,
              maxWidth
            );
            const desiredHeight = Math.max(
              contentEl.scrollHeight + titleHeight + WINDOW_CONTENT_PADDING,
              minHeight
            );
            const goldenHeight = Math.round(width / phiRef.current);
            height = clamp(Math.max(desiredHeight, goldenHeight), minHeight, maxHeight);
          }

          const position = computePosition(appId, width, height, bounds);

          next[appId] = {
            ...data,
            size: { width, height },
            position,
            userSized,
            needsLayout: false
          };
        });
        return next;
      });
    });
    return () => cancelAnimationFrame(frame);
  }, [openWindows, windowData]);

  useEffect(() => {
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver((entries) => {
      entries.forEach((entry) => {
        const windowEl = entry.target.closest('.window');
        const appId = windowEl?.dataset?.appWindow;
        if (!appId) return;
        setWindowData((prev) => {
          const data = prev[appId];
          if (!data || !data.isOpen || data.isMaximized || data.userSized) return prev;
          return {
            ...prev,
            [appId]: {
              ...data,
              needsLayout: true
            }
          };
        });
      });
    });

    openWindows.forEach((appId) => {
      const contentEl = windowRefs.current[appId]?.contentEl;
      if (contentEl) observer.observe(contentEl);
    });

    return () => observer.disconnect();
  }, [openWindows]);

  useEffect(() => {
    let nextActive = null;
    let topZ = -Infinity;
    openWindows.forEach((id) => {
      const data = windowData[id];
      if (!data || !data.isOpen || data.isMinimized) return;
      if (data.zIndex > topZ) {
        topZ = data.zIndex;
        nextActive = id;
      }
    });
    if (nextActive !== activeWindowId) {
      setActiveWindowId(nextActive);
    }
  }, [activeWindowId, openWindows, windowData]);

  useEffect(() => {
    openApp('home');
  }, []);

  const hasVisibleWindows = openWindows.some(
    (appId) => windowData[appId]?.isOpen && !windowData[appId]?.isMinimized
  );
  const isPublic = state.layer === 'public';
  const activeLayer = layerDetails[state.layer] ?? layerDetails.public;

  const handleLayerSwitch = () => {
    if (state.layer === 'protected') {
      unlockAdmin();
    } else if (state.layer === 'admin') {
      setProtected();
    }
  };

  const handleLogin = (pin) => unlock(pin);

  if (isPublic) {
    return <LandingPage onLogin={handleLogin} />;
  }

  return (
    <div className="shell">
      <header className="toolbar">
        <div className="toolbar-left">
          <div className="toolbar-brand" aria-label="Beenode">
            <span aria-hidden="true">🐝</span>
            <span>Beenode</span>
          </div>
        </div>
        <div className="toolbar-center">
          <button
            type="button"
            className="toolbar-layer-pill"
            onClick={handleLayerSwitch}
            title="Switch layer"
            aria-label={`Switch layer (current: ${activeLayer.label})`}
          >
            <span aria-hidden="true">{activeLayer.emoji}</span>
            <span>{activeLayer.label}</span>
          </button>
        </div>
        <div className="toolbar-right">
          {!isPublic && (
            <button type="button" className="toolbar-logout-btn" onClick={lock}>
              Logout
            </button>
          )}
        </div>
      </header>
      <nav className="sidebar" aria-label="App navigation">
        {apps.map((app) => {
          const isOpen = windowData[app.id]?.isOpen;
          const isActive = activeWindowId === app.id;
          return (
            <button
              key={app.id}
              className={`sidebar-icon${isOpen ? ' open' : ''}${isActive ? ' active' : ''}`}
              data-app={app.id}
              onClick={() => openApp(app.id)}
              title={app.label}
              aria-label={app.label}
            >
              <span aria-hidden="true">{app.icon}</span>
            </button>
          );
        })}
      </nav>

      <div
        ref={windowLayerRef}
        className={`window-layer${hasVisibleWindows ? ' active' : ''}`}
        id="window-layer"
      >
        <Window
          id="home"
          title="Home"
          isOpen={windowData.home.isOpen}
          isMinimized={windowData.home.isMinimized}
          isFocused={activeWindowId === 'home'}
          isMaximized={windowData.home.isMaximized}
          needsLayout={windowData.home.needsLayout}
          position={windowData.home.position}
          size={windowData.home.size}
          zIndex={windowData.home.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.home) windowRefs.current.home = {};
            windowRefs.current.home.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.home) windowRefs.current.home = {};
            windowRefs.current.home.contentEl = element;
          }}
          onClose={() => closeWindow('home')}
          onMinimize={() => minimizeWindow('home')}
          onMaximize={() => toggleMaximize('home')}
          onFocus={() => focusWindow('home')}
          onDragStart={(event) => handleDragStart(event, 'home')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'home', direction)}
        >
          <Home onCheckHealth={onCheckHealth} onSyncWallet={onSyncWallet} onGetBalance={onGetBalance} />
        </Window>
        <Window
          id="wallet"
          title="Wallet"
          isOpen={windowData.wallet.isOpen}
          isMinimized={windowData.wallet.isMinimized}
          isFocused={activeWindowId === 'wallet'}
          isMaximized={windowData.wallet.isMaximized}
          needsLayout={windowData.wallet.needsLayout}
          position={windowData.wallet.position}
          size={windowData.wallet.size}
          zIndex={windowData.wallet.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.wallet) windowRefs.current.wallet = {};
            windowRefs.current.wallet.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.wallet) windowRefs.current.wallet = {};
            windowRefs.current.wallet.contentEl = element;
          }}
          onClose={() => closeWindow('wallet')}
          onMinimize={() => minimizeWindow('wallet')}
          onMaximize={() => toggleMaximize('wallet')}
          onFocus={() => focusWindow('wallet')}
          onDragStart={(event) => handleDragStart(event, 'wallet')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'wallet', direction)}
        >
          <Wallet />
        </Window>
        <Window
          id="utxos"
          title="UTXO Explorer"
          isOpen={windowData.utxos?.isOpen}
          isMinimized={windowData.utxos?.isMinimized}
          isFocused={activeWindowId === 'utxos'}
          isMaximized={windowData.utxos?.isMaximized}
          needsLayout={windowData.utxos?.needsLayout}
          position={windowData.utxos?.position}
          size={windowData.utxos?.size}
          zIndex={windowData.utxos?.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.utxos) windowRefs.current.utxos = {};
            windowRefs.current.utxos.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.utxos) windowRefs.current.utxos = {};
            windowRefs.current.utxos.contentEl = element;
          }}
          onClose={() => closeWindow('utxos')}
          onMinimize={() => minimizeWindow('utxos')}
          onMaximize={() => toggleMaximize('utxos')}
          onFocus={() => focusWindow('utxos')}
          onDragStart={(event) => handleDragStart(event, 'utxos')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'utxos', direction)}
        >
          <UtxoExplorer />
        </Window>
        <Window
          id="txforge"
          title="TX Forge"
          isOpen={windowData.txforge?.isOpen}
          isMinimized={windowData.txforge?.isMinimized}
          isFocused={activeWindowId === 'txforge'}
          isMaximized={windowData.txforge?.isMaximized}
          needsLayout={windowData.txforge?.needsLayout}
          position={windowData.txforge?.position}
          size={windowData.txforge?.size}
          zIndex={windowData.txforge?.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.txforge) windowRefs.current.txforge = {};
            windowRefs.current.txforge.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.txforge) windowRefs.current.txforge = {};
            windowRefs.current.txforge.contentEl = element;
          }}
          onClose={() => closeWindow('txforge')}
          onMinimize={() => minimizeWindow('txforge')}
          onMaximize={() => toggleMaximize('txforge')}
          onFocus={() => focusWindow('txforge')}
          onDragStart={(event) => handleDragStart(event, 'txforge')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'txforge', direction)}
        >
          <TxForge />
        </Window>
        <Window
          id="seal"
          title="Seal Creator"
          isOpen={windowData.seal?.isOpen}
          isMinimized={windowData.seal?.isMinimized}
          isFocused={activeWindowId === 'seal'}
          isMaximized={windowData.seal?.isMaximized}
          needsLayout={windowData.seal?.needsLayout}
          position={windowData.seal?.position}
          size={windowData.seal?.size}
          zIndex={windowData.seal?.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.seal) windowRefs.current.seal = {};
            windowRefs.current.seal.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.seal) windowRefs.current.seal = {};
            windowRefs.current.seal.contentEl = element;
          }}
          onClose={() => closeWindow('seal')}
          onMinimize={() => minimizeWindow('seal')}
          onMaximize={() => toggleMaximize('seal')}
          onFocus={() => focusWindow('seal')}
          onDragStart={(event) => handleDragStart(event, 'seal')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'seal', direction)}
        >
          <SealCreator />
        </Window>
        <Window
          id="examples"
          title="Seal Examples"
          isOpen={windowData.examples?.isOpen}
          isMinimized={windowData.examples?.isMinimized}
          isFocused={activeWindowId === 'examples'}
          isMaximized={windowData.examples?.isMaximized}
          needsLayout={windowData.examples?.needsLayout}
          position={windowData.examples?.position}
          size={windowData.examples?.size}
          zIndex={windowData.examples?.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.examples) windowRefs.current.examples = {};
            windowRefs.current.examples.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.examples) windowRefs.current.examples = {};
            windowRefs.current.examples.contentEl = element;
          }}
          onClose={() => closeWindow('examples')}
          onMinimize={() => minimizeWindow('examples')}
          onMaximize={() => toggleMaximize('examples')}
          onFocus={() => focusWindow('examples')}
          onDragStart={(event) => handleDragStart(event, 'examples')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'examples', direction)}
        >
          <SealExamples />
        </Window>
        <Window
          id="reveal"
          title="Reveal Tool"
          isOpen={windowData.reveal?.isOpen}
          isMinimized={windowData.reveal?.isMinimized}
          isFocused={activeWindowId === 'reveal'}
          isMaximized={windowData.reveal?.isMaximized}
          needsLayout={windowData.reveal?.needsLayout}
          position={windowData.reveal?.position}
          size={windowData.reveal?.size}
          zIndex={windowData.reveal?.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.reveal) windowRefs.current.reveal = {};
            windowRefs.current.reveal.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.reveal) windowRefs.current.reveal = {};
            windowRefs.current.reveal.contentEl = element;
          }}
          onClose={() => closeWindow('reveal')}
          onMinimize={() => minimizeWindow('reveal')}
          onMaximize={() => toggleMaximize('reveal')}
          onFocus={() => focusWindow('reveal')}
          onDragStart={(event) => handleDragStart(event, 'reveal')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'reveal', direction)}
        >
          <RevealTool />
        </Window>
        <Window
          id="scrolls"
          title="Scrolls"
          isOpen={windowData.scrolls.isOpen}
          isMinimized={windowData.scrolls.isMinimized}
          isFocused={activeWindowId === 'scrolls'}
          isMaximized={windowData.scrolls.isMaximized}
          needsLayout={windowData.scrolls.needsLayout}
          position={windowData.scrolls.position}
          size={windowData.scrolls.size}
          zIndex={windowData.scrolls.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.scrolls) windowRefs.current.scrolls = {};
            windowRefs.current.scrolls.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.scrolls) windowRefs.current.scrolls = {};
            windowRefs.current.scrolls.contentEl = element;
          }}
          onClose={() => closeWindow('scrolls')}
          onMinimize={() => minimizeWindow('scrolls')}
          onMaximize={() => toggleMaximize('scrolls')}
          onFocus={() => focusWindow('scrolls')}
          onDragStart={(event) => handleDragStart(event, 'scrolls')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'scrolls', direction)}
        >
          <Scrolls />
        </Window>
        <Window
          id="settings"
          title="Settings"
          isOpen={windowData.settings.isOpen}
          isMinimized={windowData.settings.isMinimized}
          isFocused={activeWindowId === 'settings'}
          isMaximized={windowData.settings.isMaximized}
          needsLayout={windowData.settings.needsLayout}
          position={windowData.settings.position}
          size={windowData.settings.size}
          zIndex={windowData.settings.zIndex}
          windowRef={(element) => {
            if (!windowRefs.current.settings) windowRefs.current.settings = {};
            windowRefs.current.settings.windowEl = element;
          }}
          contentRef={(element) => {
            if (!windowRefs.current.settings) windowRefs.current.settings = {};
            windowRefs.current.settings.contentEl = element;
          }}
          onClose={() => closeWindow('settings')}
          onMinimize={() => minimizeWindow('settings')}
          onMaximize={() => toggleMaximize('settings')}
          onFocus={() => focusWindow('settings')}
          onDragStart={(event) => handleDragStart(event, 'settings')}
          onResizeStart={(event, direction) => handleResizeStart(event, 'settings', direction)}
        >
          <Settings />
        </Window>
      </div>
    </div>
  );
}

export default memo(DesktopShell);
