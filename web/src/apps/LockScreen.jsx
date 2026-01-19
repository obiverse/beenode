import { useEffect, useRef, useState } from 'react';

export default function LockScreen({ onUnlock, unlocking }) {
  const [pin, setPin] = useState('');
  const [error, setError] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [shake, setShake] = useState(false);
  const inputRef = useRef(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!shake) return;
    const timer = setTimeout(() => setShake(false), 450);
    return () => clearTimeout(timer);
  }, [shake]);

  const handleChange = (event) => {
    const nextValue = event.target.value.replace(/\D/g, '');
    setPin(nextValue);
    if (error) {
      setError('');
    }
  };

  const handleSubmit = async (event) => {
    event.preventDefault();
    if (!pin || isSubmitting || unlocking) return;
    setIsSubmitting(true);
    let success = false;
    try {
      success = await onUnlock(pin);
    } catch (error) {
      success = false;
    }
    if (!success) {
      setError('Incorrect PIN. Try again.');
      setShake(true);
      setIsSubmitting(false);
      setPin('');
      inputRef.current?.focus();
      return;
    }
  };

  return (
    <div className={`lock-screen ${unlocking ? 'unlocking' : ''}`}>
      <div className={`lock-card ${shake ? 'shake' : ''}`}>
        <div className="lock-logo">BN</div>
        <div>
          <h1 className="lock-title">Beenode OS</h1>
          <p className="lock-subtitle">Enter PIN to unlock</p>
        </div>
        <form className="lock-form" onSubmit={handleSubmit}>
          <input
            ref={inputRef}
            className="lock-input"
            type="password"
            inputMode="numeric"
            placeholder="••••••"
            value={pin}
            onChange={handleChange}
            disabled={isSubmitting || unlocking}
            autoComplete="current-password"
          />
          <div className="lock-error">{error}</div>
          <button
            className="lock-button"
            type="submit"
            disabled={!pin || isSubmitting || unlocking}
          >
            Unlock
          </button>
        </form>
      </div>
    </div>
  );
}
