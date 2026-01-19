import { useEffect, useState } from 'react';
import Card from '../components/Card.jsx';
import { api, scrollApi } from '../api/scrollApi.js';
import {
  demoSealedScrolls,
  findDemoSeal,
  loadStoredDemoSeals,
  mockUnseal
} from './DemoSealedData.js';
export default function RevealTool() {
  const [sealedPath, setSealedPath] = useState('/sealed/vault_secret');
  const [policy, setPolicy] = useState(null);
  const [evidence, setEvidence] = useState({ secret: '', identity: '' });
  const [result, setResult] = useState('');
  const [revealedContent, setRevealedContent] = useState('');
  const [loading, setLoading] = useState(false);
  const [demoOptions, setDemoOptions] = useState([]);
  const [selectedDemoPath, setSelectedDemoPath] = useState('');
  const isDemoPath = sealedPath.startsWith('/demo/');
  const dimensions = policy?.dimensions || [];
  const currentTime = new Date().toLocaleString();
  const refreshDemoOptions = () => {
    const stored = loadStoredDemoSeals();
    const storedOptions = stored.map((item) => ({
      ...item,
      label: item.label || `Saved demo: ${item.path}`
    }));
    const byPath = new Map();
    demoSealedScrolls.forEach((item) => {
      byPath.set(item.path, item);
    });
    storedOptions.forEach((item) => {
      byPath.set(item.path, {
        ...byPath.get(item.path),
        ...item
      });
    });
    setDemoOptions([...byPath.values()]);
  };

  useEffect(() => {
    refreshDemoOptions();
    const handler = () => refreshDemoOptions();
    window.addEventListener('demo-seal-created', handler);
    return () => window.removeEventListener('demo-seal-created', handler);
  }, []);
  const handleLoadPolicy = async () => {
    setLoading(true);
    setResult('');
    setRevealedContent('');
    if (isDemoPath) {
      const demoSeal = findDemoSeal(sealedPath);
      if (demoSeal) {
        setPolicy(demoSeal.policy);
      } else {
        setPolicy(null);
        setResult('Demo scroll not found. Try selecting a demo entry.');
      }
      setLoading(false);
      return;
    }
    const response = await scrollApi.scrollGet(sealedPath);
    if (response.ok) {
      const policyData = response.data?.data?.policy || response.data?.policy || response.data?.data?.seal?.policy;
      setPolicy(policyData || null);
      if (!policyData) {
        setResult('Policy not found on scroll response.');
      }
    } else {
      setResult(response.data?.error || 'Failed to load policy.');
    }
    setLoading(false);
  };

  const handleReveal = async () => {
    setLoading(true);
    setResult('');
    setRevealedContent('');
    const payload = {
      path: sealedPath,
      evidence: {
        ...(dimensions.some((dim) => dim.type === 'secret@v1') ? { secret: evidence.secret } : {}),
        ...(dimensions.some((dim) => dim.type === 'identity@v1') ? { identity: evidence.identity } : {}),
        ...(dimensions.some((dim) => dim.type === 'time@v1') ? { time: new Date().toISOString() } : {})
      }
    };
    if (isDemoPath) {
      const response = mockUnseal(sealedPath, payload.evidence);
      if (response.success) {
        setRevealedContent(response.content);
      } else {
        setResult(response.content);
      }
      setLoading(false);
      return;
    }
    const response = await api('POST', '/scroll/unseal', payload);
    if (response.ok) {
      const content = response.data?.data?.content || response.data?.content;
      if (content) {
        setRevealedContent(content);
      } else {
        setResult(JSON.stringify(response.data, null, 2));
      }
    } else {
      setResult(response.data?.error || 'Failed to unseal scroll.');
    }
    setLoading(false);
  };

  return (
    <div className="reveal-tool">
      <Card>
        <div className="seal-example-header">
          <h3>Reveal Tool</h3>
          {isDemoPath && <span className="demo-indicator">DEMO MODE</span>}
        </div>
        <label>Sealed Scroll Path</label>
        <input value={sealedPath} onChange={(event) => setSealedPath(event.target.value)} />
        <label>Load Demo</label>
        <select
          value={selectedDemoPath}
          onChange={(event) => {
            const nextPath = event.target.value;
            setSelectedDemoPath(nextPath);
            if (!nextPath) return;
            setSealedPath(nextPath);
            const demoSeal = findDemoSeal(nextPath);
            if (demoSeal) {
              setPolicy(demoSeal.policy);
              setResult('');
              setRevealedContent('');
            }
          }}
        >
          <option value="">Select a demo scroll</option>
          {demoOptions.map((item) => (
            <option key={item.path} value={item.path}>{item.label || item.path}</option>
          ))}
        </select>
        <button onClick={handleLoadPolicy} disabled={loading}>Load Policy</button>
      </Card>
      <Card>
        <h3>Policy</h3>
        {!policy && <div className="utxo-empty">Load a sealed scroll to view policy.</div>}
        {policy && (
          <>
            <div>Type: {policy.type || 'AllOf'}</div>
            {policy.threshold && <div>Threshold: {policy.threshold}</div>}
            {dimensions.length === 0 ? (
              <div className="utxo-empty">No dimensions listed.</div>
            ) : (
              <table className="utxo-table">
                <thead><tr><th>Dimension</th><th>Requirement</th></tr></thead>
                <tbody>
                  {dimensions.map((dim, index) => (
                    <tr key={`${dim.type}-${index}`}>
                      <td>{dim.type}</td>
                      <td className="utxo-txid">{JSON.stringify(dim.params || {})}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </>
        )}
      </Card>
      <Card>
        <h3>Evidence</h3>
        {dimensions.length === 0 && <div className="utxo-empty">Awaiting policy.</div>}
        {dimensions.some((dim) => dim.type === 'secret@v1') && (
          <>
            <label>PIN</label>
            <input type="password" value={evidence.secret} onChange={(event) =>
              setEvidence((prev) => ({ ...prev, secret: event.target.value }))} placeholder="Enter PIN(s), comma separated" />
          </>
        )}
        {dimensions.some((dim) => dim.type === 'time@v1') && <div>Current time: {currentTime} (auto-validated)</div>}
        {dimensions.some((dim) => dim.type === 'identity@v1') && (
          <>
            <label>Identity Challenge / Signature</label>
            <input value={evidence.identity} placeholder="Signature or hardware proof" onChange={(event) =>
              setEvidence((prev) => ({ ...prev, identity: event.target.value }))} />
          </>
        )}
        <button onClick={handleReveal} disabled={loading || !policy}>Reveal</button>
        {revealedContent && (
          <div className="reveal-success">
            <strong>🎉 Scroll unsealed!</strong>
            <div className="reveal-content">{revealedContent}</div>
          </div>
        )}
        {result && <pre>{result}</pre>}
      </Card>
    </div>
  );
}
