import { useEffect, useState } from 'react';
import Card from '../components/Card.jsx';

export default function Home({ onCheckHealth, onSyncWallet, onGetBalance }) {
  const [healthStatus, setHealthStatus] = useState(null);
  const [healthResult, setHealthResult] = useState('Click "Check Health" to connect');

  const handleCheckHealth = async () => {
    setHealthResult('Checking...');
    const response = await onCheckHealth();
    setHealthStatus(response.ok);
    setHealthResult(JSON.stringify(response.data, null, 2));
  };

  const handleSyncWallet = async () => {
    setHealthResult('Syncing wallet...');
    const response = await onSyncWallet();
    setHealthResult(JSON.stringify(response.data, null, 2));
  };

  useEffect(() => {
    handleCheckHealth();
  }, []);

  return (
    <>
      <h1>Welcome back</h1>
      <p className="subtitle">Your Beenode desktop is synced and ready.</p>
      <Card>
        <h3>Quick Actions</h3>
        <button onClick={handleCheckHealth}>Check Health</button>
        <button onClick={handleSyncWallet}>Sync Wallet</button>
        <button onClick={onGetBalance}>Refresh Balance</button>
      </Card>
      <Card>
        <h3>System Status</h3>
        <div>
          {healthStatus === null ? null : (
            <span className={`status ${healthStatus ? 'ok' : 'error'}`}>
              {healthStatus ? 'Connected' : 'Disconnected'}
            </span>
          )}
        </div>
        <pre>{healthResult}</pre>
      </Card>
    </>
  );
}
