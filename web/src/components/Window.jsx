export default function Window({
  id,
  title,
  isOpen,
  isMinimized,
  isFocused,
  isMaximized,
  needsLayout,
  position,
  size,
  zIndex,
  onClose,
  onMinimize,
  onMaximize,
  onFocus,
  onDragStart,
  onResizeStart,
  windowRef,
  contentRef,
  children
}) {
  const styles = {
    left: position?.left != null ? `${position.left}px` : undefined,
    top: position?.top != null ? `${position.top}px` : undefined,
    width: size?.width != null ? `${size.width}px` : undefined,
    height: size?.height != null ? `${size.height}px` : undefined,
    zIndex
  };

  // Don't show as active until layout is complete (prevents width animation on open)
  const isReady = isOpen && !needsLayout;

  return (
    <div
      ref={windowRef}
      className={`window${isReady ? ' active' : ''}${isMinimized ? ' minimized' : ''}${
        isFocused ? ' focused' : ''
      }${isMaximized ? ' maximized' : ''}`}
      data-app-window={id}
      style={styles}
      onMouseDown={onFocus}
    >
      <div className="window-title" onMouseDown={onDragStart} onDoubleClick={onMaximize}>
        <div className="window-controls">
          <button
            className="minimize-button"
            onMouseDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation();
              onMinimize();
            }}
          >
            −
          </button>
          <button
            className="maximize-button"
            onMouseDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation();
              onMaximize();
            }}
          >
            □
          </button>
          <button
            className="close-button"
            onMouseDown={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation();
              onClose();
            }}
          >
            ✕
          </button>
        </div>
        <span>{title}</span>
      </div>
      <div ref={contentRef} className="window-content">
        {children}
      </div>
      <div className="resize-handle n" onMouseDown={(event) => onResizeStart(event, 'n')} />
      <div className="resize-handle s" onMouseDown={(event) => onResizeStart(event, 's')} />
      <div className="resize-handle e" onMouseDown={(event) => onResizeStart(event, 'e')} />
      <div className="resize-handle w" onMouseDown={(event) => onResizeStart(event, 'w')} />
      <div className="resize-handle ne" onMouseDown={(event) => onResizeStart(event, 'ne')} />
      <div className="resize-handle nw" onMouseDown={(event) => onResizeStart(event, 'nw')} />
      <div className="resize-handle se" onMouseDown={(event) => onResizeStart(event, 'se')} />
      <div className="resize-handle sw" onMouseDown={(event) => onResizeStart(event, 'sw')} />
    </div>
  );
}
