import { sealExamples } from './SealExamples.jsx';

export const DEMO_SEAL_STORAGE_KEY = 'beenode.demo.seals';

const buildSealedAt = (index) => {
  const base = new Date('2024-06-01T12:00:00Z');
  base.setDate(base.getDate() + index);
  return base.toISOString();
};

export const demoSealedScrolls = sealExamples.map((example, index) => ({
  id: example.id,
  label: `${example.emoji} ${example.name}`,
  path: `/demo${example.scrollPath}`,
  policy: example.policy,
  sealedAt: buildSealedAt(index),
  mockContent: example.scrollData
}));

export const loadStoredDemoSeals = () => {
  if (typeof window === 'undefined') return [];
  const stored = localStorage.getItem(DEMO_SEAL_STORAGE_KEY);
  if (!stored) return [];
  try {
    const parsed = JSON.parse(stored);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item) => item && item.path && item.policy);
  } catch (error) {
    return [];
  }
};

export const saveStoredDemoSeal = (seal) => {
  const existing = loadStoredDemoSeals();
  const next = [
    ...existing.filter((item) => item.path !== seal.path),
    seal
  ];
  localStorage.setItem(DEMO_SEAL_STORAGE_KEY, JSON.stringify(next));
  return next;
};

export const findDemoSeal = (path) => {
  if (!path) return null;
  return demoSealedScrolls.find((item) => item.path === path) ||
    loadStoredDemoSeals().find((item) => item.path === path) ||
    null;
};

const parseSecretEvidence = (secret) => {
  if (!secret) return [];
  return secret
    .split(/[\s,]+/)
    .map((item) => item.trim())
    .filter(Boolean);
};

const validateDimension = (dimension, evidence) => {
  if (dimension.type === 'secret@v1') {
    const provided = parseSecretEvidence(evidence?.secret);
    return provided.includes(dimension.params?.pin);
  }
  if (dimension.type === 'time@v1') {
    const now = evidence?.time ? new Date(evidence.time) : new Date();
    if (Number.isNaN(now.getTime())) return false;
    const notBefore = dimension.params?.not_before ? new Date(dimension.params.not_before) : null;
    const notAfter = dimension.params?.not_after ? new Date(dimension.params.not_after) : null;
    if (notBefore && now < notBefore) return false;
    if (notAfter && now > notAfter) return false;
    return true;
  }
  if (dimension.type === 'identity@v1') {
    return Boolean(evidence?.identity) && evidence.identity === dimension.params?.pubkey;
  }
  if (dimension.type === 'attest@v1') {
    const attestations = Array.isArray(evidence?.attestations) ? evidence.attestations : [];
    const pubkeys = dimension.params?.pubkeys || [];
    const threshold = Number(dimension.params?.threshold) || 0;
    const matches = attestations.filter((key) => pubkeys.includes(key));
    return matches.length >= threshold && threshold > 0;
  }
  return false;
};

const validatePolicy = (policy, evidence) => {
  const dimensions = policy?.dimensions || [];
  if (dimensions.length === 0) return false;
  const matches = dimensions.map((dimension) => validateDimension(dimension, evidence));
  const matchedCount = matches.filter(Boolean).length;
  if (policy.type === 'AnyOf') {
    return matchedCount > 0;
  }
  if (policy.type === 'QuorumOf') {
    const threshold = Number(policy.threshold) || 0;
    return matchedCount >= threshold && threshold > 0;
  }
  return matchedCount === dimensions.length;
};

export const mockUnseal = (path, evidence) => {
  const seal = findDemoSeal(path);
  if (!seal) {
    return { success: false, content: 'Demo scroll not found.' };
  }
  const isValid = validatePolicy(seal.policy, evidence);
  if (!isValid) {
    return { success: false, content: 'Evidence does not satisfy the demo policy.' };
  }
  return { success: true, content: seal.mockContent };
};
