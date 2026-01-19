import { useEffect, useState } from 'react';

function formatTime(date) {
  const hours = date.getHours().toString().padStart(2, '0');
  const minutes = date.getMinutes().toString().padStart(2, '0');
  return `${hours}:${minutes}`;
}

export default function StatusBar({ networkOk }) {
  const [clock, setClock] = useState(formatTime(new Date()));

  useEffect(() => {
    const updateClock = () => setClock(formatTime(new Date()));
    updateClock();
    const interval = setInterval(updateClock, 60000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="status-bar">
      <div className="status-left">Beenode OS</div>
      <div className="status-center">{clock}</div>
      <div className="status-right">
        <span className={`network-dot ${networkOk ? 'ok' : ''}`}></span>
      </div>
    </div>
  );
}
