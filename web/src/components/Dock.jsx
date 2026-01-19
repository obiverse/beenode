export default function Dock({ apps, openApps, activeAppId, onOpenApp }) {
  const openSet = new Set(openApps);
  return (
    <nav className="dock">
      {apps.map((app) => (
        <button
          key={app.id}
          className={`dock-icon${openSet.has(app.id) ? ' open' : ''}${
            activeAppId === app.id ? ' active' : ''
          }`}
          data-app={app.id}
          title={app.label}
          onClick={() => onOpenApp(app.id)}
        >
          {app.icon}
        </button>
      ))}
    </nav>
  );
}
