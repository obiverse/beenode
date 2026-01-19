const API_BASE = '/api';

export async function api(method, path, body) {
  try {
    const options = {
      method,
      headers: { 'Content-Type': 'application/json' }
    };
    if (body) {
      options.body = JSON.stringify(body);
    }
    const response = await fetch(`${API_BASE}${path}`, options);
    const data = await response.json();
    return { ok: response.ok, data };
  } catch (error) {
    return { ok: false, data: { error: error.message } };
  }
}

export const scrollApi = {
  authStatus: () => api('GET', '/system/auth/status'),
  getAddress: () => api('GET', '/scroll/wallet/address'),
  getBalance: () => api('GET', '/scroll/wallet/balance'),
  getHealth: () => api('GET', '/health'),
  getTransactions: () => api('GET', '/scroll/wallet/transactions'),
  lock: () => api('PUT', '/system/auth/lock', {}),
  listScrolls: (prefix) => api('GET', `/scrolls?prefix=${encodeURIComponent(prefix)}`),
  scrollGet: (path) => api('GET', `/scroll${path}`),
  scrollWrite: (path, payload) => api('POST', `/scroll${path}`, payload),
  sendTransaction: (to, amountSat) => api('POST', '/scroll/wallet/send', { to, amount_sat: amountSat }),
  syncWallet: () => api('POST', '/scroll/wallet/sync', {}),
  unlock: (pin) => api('PUT', '/system/auth/unlock', { pin })
};
