import { useCallback, useEffect, useMemo, useState } from 'react';
import Card from '../components/Card.jsx';
import { api } from '../api/scrollApi.js';

export default function TxForge() {
  const [utxos, setUtxos] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [selectedUtxos, setSelectedUtxos] = useState({});
  const [outputs, setOutputs] = useState([]);
  const [outputAddress, setOutputAddress] = useState('');
  const [outputAmount, setOutputAmount] = useState('');
  const [sendResult, setSendResult] = useState('');

  const fetchUtxos = useCallback(async () => {
    setLoading(true);
    setError(null);
    const response = await api('GET', '/scroll/wallet/utxos');
    if (response.ok && response.data?.data) {
      setUtxos(response.data.data.utxos || []);
    } else {
      setError(response.data?.error || 'Failed to fetch UTXOs');
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchUtxos();
  }, [fetchUtxos]);

  const selectedTotal = useMemo(() => {
    return utxos.reduce((sum, utxo) => {
      const key = `${utxo.txid}:${utxo.vout}`;
      return selectedUtxos[key] ? sum + (utxo.amount_sat || 0) : sum;
    }, 0);
  }, [selectedUtxos, utxos]);

  const outputsTotal = useMemo(() => {
    return outputs.reduce((sum, output) => sum + output.amountSat, 0);
  }, [outputs]);

  const fee = selectedTotal - outputsTotal;

  const handleToggleUtxo = (utxoKey) => {
    setSelectedUtxos((prev) => ({ ...prev, [utxoKey]: !prev[utxoKey] }));
  };

  const handleAddOutput = () => {
    const amountSat = Number(outputAmount);
    if (!outputAddress || !amountSat) {
      return;
    }
    setOutputs((prev) => [...prev, { address: outputAddress, amountSat }]);
    setOutputAddress('');
    setOutputAmount('');
  };

  const handleRemoveOutput = (indexToRemove) => {
    setOutputs((prev) => prev.filter((_, index) => index !== indexToRemove));
  };

  const handleSend = async () => {
    if (outputs.length === 0) {
      setSendResult('Add at least one output before sending.');
      return;
    }
    setSendResult('Sending...');
    const results = [];
    for (const output of outputs) {
      const response = await api('POST', '/scroll/wallet/send', {
        to: output.address,
        amount_sat: output.amountSat
      });
      results.push(response.data);
      if (!response.ok) {
        setSendResult(JSON.stringify(response.data, null, 2));
        return;
      }
    }
    setSendResult(JSON.stringify(results, null, 2));
  };

  const truncateTxid = (txid) => (txid ? `${txid.slice(0, 8)}...` : '');

  return (
    <div className="tx-forge">
      <Card>
        <div className="utxo-header">
          <h3>UTXO Selector</h3>
          <button onClick={fetchUtxos} disabled={loading}>
            {loading ? 'Loading...' : 'Refresh'}
          </button>
        </div>

        {error && <div className="utxo-error">{error}</div>}

        {!error && !loading && utxos.length === 0 && (
          <div className="utxo-empty">No UTXOs found. Sync wallet first.</div>
        )}

        {!error && utxos.length > 0 && (
          <table className="utxo-table">
            <thead>
              <tr>
                <th></th>
                <th>TXID</th>
                <th>Vout</th>
                <th>Amount (sats)</th>
              </tr>
            </thead>
            <tbody>
              {utxos.map((utxo) => {
                const key = `${utxo.txid}:${utxo.vout}`;
                return (
                  <tr key={key} className="utxo-row">
                    <td>
                      <input
                        type="checkbox"
                        checked={Boolean(selectedUtxos[key])}
                        onChange={() => handleToggleUtxo(key)}
                      />
                    </td>
                    <td className="utxo-txid" title={utxo.txid}>
                      {truncateTxid(utxo.txid)}
                    </td>
                    <td className="utxo-vout">{utxo.vout}</td>
                    <td className="utxo-amount">{utxo.amount_sat?.toLocaleString()}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </Card>

      <Card>
        <h3>Selected Total</h3>
        <div className="balance">
          {selectedTotal.toLocaleString()} <span className="sats">sats</span>
        </div>
      </Card>

      <div className="grid">
        <Card>
          <h3>Output Builder</h3>
          <label>To Address:</label>
          <input
            value={outputAddress}
            onChange={(event) => setOutputAddress(event.target.value)}
            placeholder="bcrt1q..."
          />
          <label>Amount (sats):</label>
          <input
            type="number"
            value={outputAmount}
            onChange={(event) => setOutputAmount(event.target.value)}
          />
          <button onClick={handleAddOutput}>Add Output</button>

          {outputs.length > 0 && (
            <table className="utxo-table">
              <thead>
                <tr>
                  <th>Address</th>
                  <th>Amount</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {outputs.map((output, index) => (
                  <tr key={`${output.address}-${index}`}>
                    <td className="utxo-txid" title={output.address}>
                      {output.address.slice(0, 12)}...
                    </td>
                    <td className="utxo-amount">{output.amountSat.toLocaleString()}</td>
                    <td>
                      <button onClick={() => handleRemoveOutput(index)}>Remove</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Card>

        <Card>
          <h3>Fee Summary</h3>
          <div>Total Input: {selectedTotal.toLocaleString()} sats</div>
          <div>Total Output: {outputsTotal.toLocaleString()} sats</div>
          <div>Fee: {fee.toLocaleString()} sats</div>
          <button onClick={handleSend}>Send Transaction</button>
          {sendResult && <pre>{sendResult}</pre>}
        </Card>
      </div>
    </div>
  );
}
