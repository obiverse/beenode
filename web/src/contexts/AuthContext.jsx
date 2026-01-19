import { createContext, useContext, useEffect, useMemo, useState } from 'react';

const STORAGE_KEY = 'beenode_layer';
const DEV_PIN = '000000';
const PROTECTED_PIN = '1234';
const defaultState = { layer: 'public', pin: null, error: null };

const AuthContext = createContext({
  state: defaultState,
  unlock: () => {},
  unlockAdmin: () => {},
  setProtected: () => {},
  lock: () => {}
});

export const AuthProvider = ({ children }) => {
  const [state, setState] = useState(defaultState);

  useEffect(() => {
    localStorage.removeItem(STORAGE_KEY);
    const stored = sessionStorage.getItem(STORAGE_KEY);
    if (stored === 'protected' || stored === 'admin') {
      setState((prev) => ({ ...prev, layer: stored }));
    }
  }, []);

  useEffect(() => {
    if (state.layer === 'protected' || state.layer === 'admin') {
      sessionStorage.setItem(STORAGE_KEY, state.layer);
    } else {
      sessionStorage.removeItem(STORAGE_KEY);
    }
  }, [state.layer]);

  const unlock = (pin) => {
    const expectedPin = import.meta.env.DEV ? DEV_PIN : PROTECTED_PIN;
    if (pin === expectedPin) {
      setState({ layer: 'protected', pin, error: null });
      return true;
    }
    setState((prev) => ({ ...prev, error: 'Invalid PIN' }));
    return false;
  };
  const unlockAdmin = () => setState((prev) => ({ ...prev, layer: 'admin', error: null }));
  const setProtected = () => setState((prev) => ({ ...prev, layer: 'protected', error: null }));
  const lock = () => setState(defaultState);

  const value = useMemo(() => ({ state, unlock, unlockAdmin, setProtected, lock }), [state]);

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

export const useAuth = () => useContext(AuthContext);
