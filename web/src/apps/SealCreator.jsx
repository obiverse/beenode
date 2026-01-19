import { useEffect, useMemo, useState } from 'react';
import Card from '../components/Card.jsx';
import { api } from '../api/scrollApi.js';
import { sealExamples, SEAL_EXAMPLE_STORAGE_KEY } from './SealExamples.jsx';
import { saveStoredDemoSeal } from './DemoSealedData.js';
const defaultParams = { not_before: '', not_after: '', pin: '', pubkey: '', attestThreshold: '', attestPubkeys: '' };
const dimensionOptions = ['time@v1', 'secret@v1', 'identity@v1', 'attest@v1'];
export default function SealCreator() {
  const [scrollPath, setScrollPath] = useState('/vault/secret');
  const [selectedExampleId, setSelectedExampleId] = useState('');
  const [policyType, setPolicyType] = useState('AllOf');
  const [threshold, setThreshold] = useState(2);
  const [dimensions, setDimensions] = useState([]);
  const [newDimType, setNewDimType] = useState('time@v1');
  const [newDimParams, setNewDimParams] = useState(defaultParams);
  const [result, setResult] = useState('');
  const selectedExample = useMemo(
    () => sealExamples.find((example) => example.id === selectedExampleId),
    [selectedExampleId]
  );
  const policyPreview = useMemo(() => {
    const policy = {
      type: policyType,
      ...(policyType === 'QuorumOf' ? { threshold: Number(threshold) || 0 } : {}),
      dimensions
    };
    return JSON.stringify(policy, null, 2);
  }, [dimensions, policyType, threshold]);
  const updateParam = (key) => (event) =>
    setNewDimParams((prev) => ({ ...prev, [key]: event.target.value }));

  const applyExample = (example) => {
    if (!example) return;
    const nextPolicy = example.policy;
    setScrollPath(example.scrollPath || '/vault/secret');
    setPolicyType(nextPolicy.type);
    setThreshold(nextPolicy.threshold ?? 2);
    setDimensions(nextPolicy.dimensions.map((dimension) => ({
      type: dimension.type,
      params: { ...dimension.params }
    })));
    setNewDimType(nextPolicy.dimensions[0]?.type || 'time@v1');
    setNewDimParams(defaultParams);
    setResult('');
  };

  useEffect(() => {
    const storedExampleId = localStorage.getItem(SEAL_EXAMPLE_STORAGE_KEY);
    if (storedExampleId) {
      const example = sealExamples.find((item) => item.id === storedExampleId);
      if (example) {
        setSelectedExampleId(example.id);
        applyExample(example);
      }
    }
    const handler = (event) => {
      const nextId = event.detail?.id || event.detail;
      const example = sealExamples.find((item) => item.id === nextId);
      if (example) {
        setSelectedExampleId(example.id);
        applyExample(example);
      }
    };
    window.addEventListener('seal-example-selected', handler);
    return () => window.removeEventListener('seal-example-selected', handler);
  }, []);

  const handleAddDimension = () => {
    let params = {};
    if (newDimType === 'time@v1') {
      params = { not_before: newDimParams.not_before, not_after: newDimParams.not_after };
    } else if (newDimType === 'secret@v1') {
      if (!newDimParams.pin) return setResult('PIN is required for secret@v1');
      params = { pin: newDimParams.pin };
    } else if (newDimType === 'identity@v1') {
      if (!newDimParams.pubkey) return setResult('Pubkey is required for identity@v1');
      params = { pubkey: newDimParams.pubkey };
    } else if (newDimType === 'attest@v1') {
      const thresholdValue = Number(newDimParams.attestThreshold);
      const pubkeys = newDimParams.attestPubkeys.split(',').map((key) => key.trim()).filter(Boolean);
      if (!thresholdValue || pubkeys.length === 0) return setResult('Threshold and pubkeys required for attest@v1');
      params = { threshold: thresholdValue, pubkeys };
    }
    setDimensions((prev) => [...prev, { type: newDimType, params }]);
    setNewDimParams(defaultParams);
  };

  const handleCreateSeal = async () => {
    const payload = { path: scrollPath, policy: JSON.parse(policyPreview) };
    if (scrollPath.startsWith('/demo/')) {
      saveDemoSeal(scrollPath, payload.policy);
      return;
    }
    setResult('Creating seal...');
    const response = await api('POST', '/scroll/seal', payload);
    setResult(JSON.stringify(response.data, null, 2));
  };

  const normalizeDemoPath = (path) => {
    if (path.startsWith('/demo/')) return path;
    if (path.startsWith('/')) return `/demo${path}`;
    return `/demo/${path}`;
  };

  const saveDemoSeal = (path, policy) => {
    const demoPath = normalizeDemoPath(path);
    const mockContent = selectedExample?.scrollData || `Demo secret stored at ${demoPath}.`;
    saveStoredDemoSeal({
      path: demoPath,
      policy,
      sealedAt: new Date().toISOString(),
      mockContent,
      label: selectedExample ? `${selectedExample.emoji} ${selectedExample.name} (saved)` : `Saved demo: ${demoPath}`
    });
    window.dispatchEvent(new CustomEvent('demo-seal-created', { detail: { path: demoPath } }));
    setScrollPath(demoPath);
    setResult(`Demo seal saved. Use ${demoPath} in Reveal Tool.`);
  };

  const renderParamsFields = () => {
    if (newDimType === 'time@v1') {
      return (
        <>
          <label>Not Before</label><input type="datetime-local" value={newDimParams.not_before} onChange={updateParam('not_before')} />
          <label>Not After</label><input type="datetime-local" value={newDimParams.not_after} onChange={updateParam('not_after')} />
        </>
      );
    }
    if (newDimType === 'secret@v1') {
      return (
        <>
          <label>PIN</label><input type="password" value={newDimParams.pin} onChange={updateParam('pin')} />
        </>
      );
    }
    if (newDimType === 'identity@v1') {
      return (
        <>
          <label>Pubkey</label><input value={newDimParams.pubkey} onChange={updateParam('pubkey')} placeholder="npub... or hex" />
        </>
      );
    }
    return (
      <>
        <label>Threshold</label><input type="number" value={newDimParams.attestThreshold} onChange={updateParam('attestThreshold')} />
        <label>Pubkeys (comma separated)</label><input value={newDimParams.attestPubkeys} onChange={updateParam('attestPubkeys')} />
      </>
    );
  };

  return (
    <div className="seal-creator">
      <Card>
        <h3>Seal Creator</h3>
        <label>Examples</label>
        <select
          value={selectedExampleId}
          onChange={(event) => {
            const nextId = event.target.value;
            setSelectedExampleId(nextId);
            const example = sealExamples.find((item) => item.id === nextId);
            if (example) {
              localStorage.setItem(SEAL_EXAMPLE_STORAGE_KEY, example.id);
              applyExample(example);
            }
          }}
        >
          <option value="">Select an example</option>
          {sealExamples.map((example) => (
            <option key={example.id} value={example.id}>{example.emoji} {example.name}</option>
          ))}
        </select>
        {selectedExample && (
          <div className="seal-example-info">
            <strong>Use case:</strong> {selectedExample.description}
          </div>
        )}
        <label>Scroll Path</label><input value={scrollPath} onChange={(event) => setScrollPath(event.target.value)} />
        <label>Policy Type</label>
        <select value={policyType} onChange={(event) => setPolicyType(event.target.value)}>
          <option value="AllOf">AllOf</option>
          <option value="AnyOf">AnyOf</option>
          <option value="QuorumOf">QuorumOf</option>
        </select>
        {policyType === 'QuorumOf' && (
          <>
            <label>Threshold</label><input type="number" value={threshold} onChange={(event) => setThreshold(event.target.value)} />
          </>
        )}
      </Card>
      <div className="grid">
        <Card>
          <h3>Add Dimension</h3>
          <label>Dimension Type</label>
          <select
            value={newDimType}
            onChange={(event) => {
              setNewDimType(event.target.value);
              setNewDimParams(defaultParams);
            }}
          >
            {dimensionOptions.map((option) => (
              <option key={option} value={option}>{option}</option>
            ))}
          </select>
          {renderParamsFields()}
          <button onClick={handleAddDimension}>Add Dimension</button>
        </Card>
        <Card>
          <h3>Policy Preview</h3>
          <pre>{policyPreview}</pre>
        </Card>
      </div>
      <Card>
        <h3>Added Dimensions</h3>
        {dimensions.length === 0 ? (
          <div className="utxo-empty">No dimensions added.</div>
        ) : (
          <table className="utxo-table">
            <thead><tr><th>Type</th><th>Params</th><th></th></tr></thead>
            <tbody>
              {dimensions.map((dimension, index) => (
                <tr key={`${dimension.type}-${index}`}>
                  <td>{dimension.type}</td>
                  <td className="utxo-txid">{JSON.stringify(dimension.params)}</td>
                  <td>
                    <button onClick={() => setDimensions((prev) => prev.filter((_, i) => i !== index))}>Remove</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </Card>
      <Card>
        <button onClick={handleCreateSeal}>Create Seal</button>
        <button onClick={() => saveDemoSeal(scrollPath, JSON.parse(policyPreview))}>Create Demo Seal</button>
        {result && <pre>{result}</pre>}
      </Card>
    </div>
  );
}
