import { useState } from 'react';
import Card from '../components/Card.jsx';
import { scrollApi } from '../api/scrollApi.js';
import { useBalance } from '../contexts/BalanceContext.js';

export default function Wallet() {
  const { balance, refreshBalance } = useBalance();
  const [addressResult, setAddressResult] = useState('Click to generate');
  const [sendResult, setSendResult] = useState('');
  const [txsResult, setTxsResult] = useState('Click to load');
  const [sendTo, setSendTo] = useState('');
  const [sendAmount, setSendAmount] = useState(10000);

  const handleGetAddress = async () => {
    setAddressResult('Generating...');
    const response = await scrollApi.getAddress();
    if (response.ok && response.data?.data) {
      setAddressResult(response.data.data.address);
    } else {
      setAddressResult(JSON.stringify(response.data, null, 2));
    }
  };

  const handleSend = async () => {
    if (!sendTo || !sendAmount) {
      setSendResult('Please fill in address and amount');
      return;
    }
    setSendResult('Sending...');
    const response = await scrollApi.sendTransaction(sendTo, Number(sendAmount));
    setSendResult(JSON.stringify(response.data, null, 2));
    if (response.ok) {
      setTimeout(refreshBalance, 2000);
    }
  };

  const handleGetTxs = async () => {
    setTxsResult('Loading...');
    const response = await scrollApi.getTransactions();
    if (response.ok && response.data?.data) {
      const txs = response.data.data.transactions || [];
      if (txs.length === 0) {
        setTxsResult('No transactions yet');
      } else {
        setTxsResult(
          txs
            .map((tx) => {
              const amount = Math.abs(tx.received - tx.sent);
              return `${tx.txid.slice(0, 8)}... ${tx.sent > 0 ? '-' : '+'}${amount} sats`;
            })
            .join('\n')
        );
      }
    } else {
      setTxsResult(JSON.stringify(response.data, null, 2));
    }
  };

  const balanceMarkup = () => {
    if (balance.error) {
      return <span style={{ color: '#e74c3c' }}>Error</span>;
    }
    if (balance.confirmed === null) {
      return (
        <>
          -- <span className="sats">sats</span>
        </>
      );
    }
    return (
      <>
        {balance.confirmed.toLocaleString()} <span className="sats">sats</span>
        {balance.pending > 0 ? (
          <span style={{ color: '#888' }}> (+{balance.pending} pending)</span>
        ) : null}
      </>
    );
  };

  return (
    <>
      <Card>
        <h3>Balance</h3>
        <div className="balance">{balanceMarkup()}</div>
        <button onClick={refreshBalance}>Refresh Balance</button>
      </Card>

      <div className="grid">
        <Card>
          <h3>Receive Address</h3>
          <button onClick={handleGetAddress}>Get New Address</button>
          <pre>{addressResult}</pre>
        </Card>

        <Card>
          <h3>Send Bitcoin</h3>
          <label>To Address:</label>
          <input
            value={sendTo}
            onChange={(event) => setSendTo(event.target.value)}
            placeholder="bcrt1q..."
          />
          <label>Amount (sats):</label>
          <input
            type="number"
            value={sendAmount}
            onChange={(event) => setSendAmount(event.target.value)}
          />
          <button onClick={handleSend}>Send Transaction</button>
          <pre>{sendResult}</pre>
        </Card>
      </div>

      <Card>
        <h3>Recent Transactions</h3>
        <button onClick={handleGetTxs}>Load Transactions</button>
        <pre>{txsResult}</pre>
      </Card>
    </>
  );
}
