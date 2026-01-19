import { useCallback, useEffect, useState } from 'react';
import Card from '../components/Card.jsx';
import { api } from '../api/scrollApi.js';

export default function UtxoExplorer() {
  const [utxos, setUtxos] = useState([]);
  const [totalSat, setTotalSat] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  const satsToBtc = (sats) => (sats / 100000000).toFixed(8);

  const fetchUtxos = useCallback(async () => {
    setLoading(true);
    setError(null);
    const response = await api('GET', '/scroll/wallet/utxos');
    if (response.ok && response.data?.data) {
      setUtxos(response.data.data.utxos || []);
      setTotalSat(response.data.data.total_sat || 0);
    } else {
      setError(response.data?.error || 'Failed to fetch UTXOs');
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    fetchUtxos();
  }, [fetchUtxos]);

  const truncateTxid = (txid) => txid ? `${txid.slice(0, 8)}...` : '';

  return (
    <div className="utxo-explorer">
      <Card>
        <div className="utxo-header">
          <h3>Unspent Outputs</h3>
          <button onClick={fetchUtxos} disabled={loading}>
            {loading ? 'Loading...' : 'Refresh'}
          </button>
        </div>

        {error && <div className="utxo-error">{error}</div>}

        {!error && !loading && utxos.length === 0 && (
          <div className="utxo-empty">No UTXOs found. Sync wallet first.</div>
        )}

        {!error && utxos.length > 0 && (
          <>
            <table className="utxo-table">
              <thead>
                <tr>
                  <th>TXID</th>
                  <th>Vout</th>
                  <th>Amount (BTC)</th>
                  <th>Type</th>
                </tr>
              </thead>
              <tbody>
                {utxos.map((utxo) => (
                  <tr key={`${utxo.txid}:${utxo.vout}`} className="utxo-row">
                    <td className="utxo-txid" title={utxo.txid}>
                      {truncateTxid(utxo.txid)}
                    </td>
                    <td className="utxo-vout">{utxo.vout}</td>
                    <td className="utxo-amount">{satsToBtc(utxo.amount_sat)}</td>
                    <td className="utxo-type">
                      <span className={utxo.is_change ? 'change' : 'external'}>
                        {utxo.is_change ? 'Change' : 'External'}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>

            <div className="utxo-summary">
              <span className="utxo-count">{utxos.length} UTXO{utxos.length !== 1 ? 's' : ''}</span>
              <span className="utxo-total">Total: {satsToBtc(totalSat)} BTC</span>
            </div>
          </>
        )}
      </Card>
    </div>
  );
}
