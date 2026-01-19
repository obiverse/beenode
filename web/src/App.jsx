import { useCallback, useEffect, useState } from 'react';
import LockScreen from './apps/LockScreen.jsx';
import DesktopShell from './components/DesktopShell.jsx';
import StatusBar from './components/StatusBar.jsx';
import { BalanceProvider } from './contexts/BalanceContext.js';
import { api, scrollApi } from './api/scrollApi.js';

const emptyBalance = {
  confirmed: null,
  pending: 0,
  error: false
};

export default function App() {
  const [networkOk, setNetworkOk] = useState(false);
  const [balance, setBalance] = useState(emptyBalance);
  const [authLocked, setAuthLocked] = useState(false);
  const [authReady, setAuthReady] = useState(false);
  const [unlocking, setUnlocking] = useState(false);

  const getBalance = useCallback(async () => {
    const response = await api('GET', '/scroll/wallet/balance');
    if (response.ok && response.data?.data) {
      setBalance({
        confirmed: response.data.data.confirmed,
        pending: response.data.data.pending ?? 0,
        error: false
      });
    } else {
      setBalance((prev) => ({ ...prev, error: true }));
    }
  }, []);

  const checkHealth = useCallback(async () => {
    const response = await api('GET', '/health');
    setNetworkOk(response.ok);
    if (response.ok) {
      getBalance();
    }
    return response;
  }, [getBalance]);

  const syncWallet = useCallback(async () => {
    const response = await api('POST', '/scroll/wallet/sync', {});
    if (response.ok) {
      setTimeout(getBalance, 1000);
    }
    return response;
  }, [getBalance]);

  const fetchAuthStatus = useCallback(async () => {
    const response = await scrollApi.authStatus();
    if (response.ok && response.data) {
      setAuthLocked(Boolean(response.data.locked));
    }
    setAuthReady(true);
  }, []);

  const handleUnlock = useCallback(async (pin) => {
    const response = await scrollApi.unlock(pin);
    const success = response.ok && response.data?.success;
    if (success) {
      setUnlocking(true);
      setTimeout(() => {
        setAuthLocked(false);
        setUnlocking(false);
      }, 300);
    }
    return success;
  }, []);

  useEffect(() => {
    fetchAuthStatus();
  }, [fetchAuthStatus]);

  if (!authReady) {
    return <div className="os-shell" />;
  }

  if (authLocked) {
    return <LockScreen onUnlock={handleUnlock} unlocking={unlocking} />;
  }

  return (
    <div className="os-shell">
      <StatusBar networkOk={networkOk} />
      <BalanceProvider value={{ balance, refreshBalance: getBalance }}>
        <DesktopShell
          onCheckHealth={checkHealth}
          onSyncWallet={syncWallet}
          onGetBalance={getBalance}
        />
      </BalanceProvider>
    </div>
  );
}
