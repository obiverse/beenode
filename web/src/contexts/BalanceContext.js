import { createContext, useContext } from 'react';

const BalanceContext = createContext({
  balance: { confirmed: null, pending: 0, error: false },
  refreshBalance: () => {}
});

export const BalanceProvider = BalanceContext.Provider;

export const useBalance = () => useContext(BalanceContext);
