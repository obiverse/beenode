import { useState } from 'react';
import { useAuth } from '../contexts/AuthContext.jsx';

export default function LandingPage({ onLogin }) {
  const { state } = useAuth();
  const isDev = import.meta.env.DEV;
  const [pin, setPin] = useState(isDev ? '000000' : '');

  const handleSubmit = (event) => {
    event.preventDefault();
    onLogin(pin);
  };

  return (
    <div style={{ minHeight: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '40px' }}>
      <div
        style={{ width: 'min(520px, 92vw)', padding: '36px 40px', background: 'var(--glass)', border: '1px solid var(--glass-border)', borderRadius: 24, boxShadow: '0 24px 48px var(--shadow)', backdropFilter: 'blur(18px)', textAlign: 'center', display: 'flex', flexDirection: 'column', gap: 16 }}
      >
        {isDev && (
          <span
            style={{ alignSelf: 'center', fontSize: 12, padding: '4px 10px', borderRadius: 999, background: 'rgba(247, 183, 49, 0.12)', border: '1px solid rgba(247, 183, 49, 0.4)', color: 'var(--accent)' }}
          >
            Dev Mode
          </span>
        )}
        <h1 style={{ margin: 0, fontSize: 28 }}>Welcome to Beenode</h1>
        <div style={{ fontSize: 16, color: 'var(--text-muted)' }}>The Lab for Sovereign Computing</div>
        <p style={{ margin: 0, color: 'var(--text-muted)', lineHeight: 1.5 }}>
          Beenode gathers Bitcoin, Lightning, Nostr, and 9S craft into one quiet lab. Explore
          sovereign tooling while your node hums in the background.
        </p>
        <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: 12, marginTop: 4 }}>
          <input
            type="password"
            value={pin}
            onChange={(event) => setPin(event.target.value)}
            placeholder="Enter PIN"
            style={{ padding: '12px 16px', fontSize: 16, borderRadius: 12, border: '1px solid var(--glass-border)', background: 'rgba(6, 8, 16, 0.6)', color: 'var(--text-primary)', textAlign: 'center', letterSpacing: 4 }}
          />
          <button
            type="submit"
            style={{ padding: '12px 16px', borderRadius: 12, border: '1px solid rgba(247, 183, 49, 0.4)', background: 'rgba(247, 183, 49, 0.18)', color: 'var(--text-primary)', fontSize: 16, cursor: 'pointer' }}
          >
            Enter Lab
          </button>
        </form>
        <div style={{ minHeight: 18, fontSize: 12, color: '#ff7675' }}>{state.error ?? ''}</div>
      </div>
    </div>
  );
}
