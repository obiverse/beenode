import { useState } from 'react';
import Card from '../components/Card.jsx';

export const SEAL_EXAMPLE_STORAGE_KEY = 'beenode.seal.example';

export const sealExamples = [
  {
    id: 'PIN_ONLY',
    name: 'PIN Only',
    emoji: '🔢',
    tagline: 'Protect with a 4-digit PIN.',
    description: 'A simple shared-secret lock for notes you want to keep private but easy to access. Great for quick vault entries or shared team lockers where a single PIN is enough.',
    scrollPath: '/vault/pin-only',
    scrollData: 'Emergency access code: 8742-ALPHA.',
    unseal: 'Enter the 4-digit PIN 4829 to unlock the scroll.',
    policy: {
      type: 'AllOf',
      dimensions: [
        { type: 'secret@v1', params: { pin: '4829' } }
      ]
    }
  },
  {
    id: 'TIME_LOCK',
    name: 'Time Lock',
    emoji: '🎂',
    tagline: 'Reveal on my birthday.',
    description: 'Keeps the scroll sealed until a future date. Use it for surprise messages, delayed releases, or any content that should stay hidden until a milestone.',
    scrollPath: '/vault/birthday-reveal',
    scrollData: 'Birthday surprise: Reserve table for two at 7pm.',
    unseal: 'Wait until 2031-07-18 09:00 before attempting to unseal.',
    policy: {
      type: 'AllOf',
      dimensions: [
        { type: 'time@v1', params: { not_before: '2031-07-18T09:00', not_after: '' } }
      ]
    }
  },
  {
    id: 'TIME_WINDOW',
    name: 'Time Window',
    emoji: '🕰️',
    tagline: 'Access during business hours.',
    description: 'Allows access only within a defined window. Ideal for on-call runbooks, timed promotions, or business-hour secrets that should not be visible outside the window.',
    scrollPath: '/vault/business-hours',
    scrollData: 'On-call checklist and pager escalation notes.',
    unseal: 'Unseal between 2030-05-01 09:00 and 2030-05-01 17:00.',
    policy: {
      type: 'AllOf',
      dimensions: [
        { type: 'time@v1', params: { not_before: '2030-05-01T09:00', not_after: '2030-05-01T17:00' } }
      ]
    }
  },
  {
    id: 'PIN_AND_TIME',
    name: 'PIN + Time',
    emoji: '⏳',
    tagline: 'PIN that expires.',
    description: 'Combines a shared PIN with a time limit so the code only works before a deadline. Helpful for short-term access like temporary contractors or staged reveals.',
    scrollPath: '/vault/temp-pin',
    scrollData: 'Temp access note: contractor VPN details.',
    unseal: 'Enter PIN 7316 before 2029-12-31 23:59.',
    policy: {
      type: 'AllOf',
      dimensions: [
        { type: 'secret@v1', params: { pin: '7316' } },
        { type: 'time@v1', params: { not_before: '', not_after: '2029-12-31T23:59' } }
      ]
    }
  },
  {
    id: 'PIN_OR_IDENTITY',
    name: 'PIN or Identity',
    emoji: '🗝️',
    tagline: 'Password or hardware key.',
    description: 'Lets someone unlock with either a PIN or a known public key. Great for allowing a backup hardware key while still supporting a memorized secret.',
    scrollPath: '/vault/dual-access',
    scrollData: 'Server reboot checklist for the on-call team.',
    unseal: 'Provide PIN 9901 or sign with the listed public key.',
    policy: {
      type: 'AnyOf',
      dimensions: [
        { type: 'secret@v1', params: { pin: '9901' } },
        { type: 'identity@v1', params: { pubkey: '03b21f7d9f2e9d3c7a6a5d1b4c2a0f9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3' } }
      ]
    }
  },
  {
    id: 'RECOVERY_2OF3',
    name: 'Recovery 2-of-3',
    emoji: '🧩',
    tagline: 'Social recovery.',
    description: 'Requires any two trusted PINs to unseal the data. It mimics social recovery workflows where multiple people must agree to unlock.',
    scrollPath: '/vault/social-recovery',
    scrollData: 'Wallet recovery checklist and seed phrase split locations.',
    unseal: 'Enter any two of the three PINs: 1209, 5577, 8890.',
    policy: {
      type: 'QuorumOf',
      threshold: 2,
      dimensions: [
        { type: 'secret@v1', params: { pin: '1209' } },
        { type: 'secret@v1', params: { pin: '5577' } },
        { type: 'secret@v1', params: { pin: '8890' } }
      ]
    }
  },
  {
    id: 'DEAD_MANS_SWITCH',
    name: "Dead Man's Switch",
    emoji: '🛟',
    tagline: "Auto-reveal if I don't check in.",
    description: 'Either a trusted PIN unlocks immediately or the seal opens automatically after a deadline. Useful for contingency plans and safety check-ins.',
    scrollPath: '/vault/check-in',
    scrollData: 'Emergency contact list and contingency instructions.',
    unseal: 'Use PIN 4455 anytime, or wait until 2032-02-01 08:00.',
    policy: {
      type: 'AnyOf',
      dimensions: [
        { type: 'secret@v1', params: { pin: '4455' } },
        { type: 'time@v1', params: { not_before: '2032-02-01T08:00', not_after: '' } }
      ]
    }
  },
  {
    id: 'INHERITANCE',
    name: 'Inheritance',
    emoji: '🏛️',
    tagline: 'Estate planning.',
    description: 'Provides a fallback that either waits for a future date or requires trustee consensus. It is ideal for estate plans or long-term custody agreements.',
    scrollPath: '/vault/estate-plan',
    scrollData: 'Estate instructions and account inventory.',
    unseal: 'Wait until 2035-01-01 12:00 or gather 2 of 3 trustee attestations.',
    policy: {
      type: 'AnyOf',
      dimensions: [
        { type: 'time@v1', params: { not_before: '2035-01-01T12:00', not_after: '' } },
        {
          type: 'attest@v1',
          params: {
            threshold: 2,
            pubkeys: [
              'npub1trusteealice0l9xk5txpuwq4u0ls4u7v6m4v3l0fsw4z8kxs0g5s9d9q4m',
              'npub1trusteeberndf9zjv8a7s0d5t7s3m6l9p0f4r2k6j8h9c4s2d1f3g8h7j5k',
              'npub1trusteeclara2q0m5z9x8c7v6b5n4m3l2k1j0h9g8f7d6s5a4p3o2i1u'
            ]
          }
        }
      ]
    }
  }
];

export default function SealExamples() {
  const [lastCopiedId, setLastCopiedId] = useState('');

  const handleTryExample = (example) => {
    localStorage.setItem(SEAL_EXAMPLE_STORAGE_KEY, example.id);
    window.dispatchEvent(new CustomEvent('seal-example-selected', { detail: { id: example.id } }));
    setLastCopiedId(example.id);
  };

  return (
    <div className="seal-examples">
      <Card>
        <h3>Sealing & Unsealing Tour</h3>
        <div className="subtitle">Pick a scenario to explore how sealing policies behave in real life.</div>
        {lastCopiedId && (
          <div className="seal-example-info">
            Example loaded into Seal Creator: {lastCopiedId}
          </div>
        )}
      </Card>
      <div className="grid seal-examples-grid">
        {sealExamples.map((example) => (
          <Card key={example.id}>
            <div className="seal-example-header">
              <h3>{example.emoji} {example.name}</h3>
              <span className="seal-example-tag">{example.tagline}</span>
            </div>
            <p>{example.description}</p>
            <div className="seal-example-meta"><strong>Sample scroll:</strong> {example.scrollData}</div>
            <div className="seal-example-meta"><strong>How to unseal:</strong> {example.unseal}</div>
            <div className="seal-example-label">Policy JSON</div>
            <pre>{JSON.stringify(example.policy, null, 2)}</pre>
            <div className="seal-example-actions">
              <button onClick={() => handleTryExample(example)}>Try it</button>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}
