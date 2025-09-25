import { getAudioContext, registerSound } from './index.mjs';
import { clamp, getSoundIndex, valueToMidi } from './util.mjs';
import {
  destroyAudioWorkletNode,
  getADSRValues,
  getFrequencyFromValue,
  getParamADSR,
  getPitchEnvelope,
  getVibratoOscillator,
  getWorklet,
  webAudioTimeout,
} from './helpers.mjs';
import { logger } from './logger.mjs';

const WT_MAX_MIP_LEVELS = 6;
export const WarpMode = Object.freeze({
  NONE: 0,
  ASYM: 1,
  MIRROR: 2,
  BENDP: 3,
  BENDM: 4,
  BENDMP: 5,
  SYNC: 6,
  QUANT: 7,
  FOLD: 8,
  PWM: 9,
  ORBIT: 10,
  SPIN: 11,
  CHAOS: 12,
  PRIMES: 13,
  BINARY: 14,
  BROWNIAN: 15,
  RECIPROCAL: 16,
  WORMHOLE: 17,
  LOGISTIC: 18,
  SIGMOID: 19,
  FRACTAL: 20,
  FLIP: 21,
});

async function loadWavetableFrames(url, label, frameLen = 256) {
  const ac = getAudioContext();
  const buf = await loadBuffer(url, ac, label);
  const ch0 = buf.getChannelData(0);
  const total = ch0.length;
  const numFrames = Math.floor(total / frameLen);
  const frames = new Array(numFrames);
  for (let i = 0; i < numFrames; i++) {
    const start = i * frameLen;
    frames[i] = ch0.subarray(start, start + frameLen);
  }

  // build mipmaps
  const mipmaps = [frames];
  let levelFrames = frames;
  for (let level = 1; level < WT_MAX_MIP_LEVELS; level++) {
    const prevLen = levelFrames[0].length;
    if (prevLen <= 32) break;
    const nextLen = prevLen >> 1;
    const next = levelFrames.map((src) => {
      const out = new Float32Array(nextLen);
      for (let j = 0; j < nextLen; j++) {
        out[j] = (src[2 * j] + src[2 * j + 1]) / 2;
      }
      return out;
    });
    mipmaps.push(next);
    levelFrames = next;
  }
  return { frames, mipmaps, frameLen, numFrames };
}

const loadCache = {};

function humanFileSize(bytes, si) {
  var thresh = si ? 1000 : 1024;
  if (bytes < thresh) return bytes + ' B';
  var units = si
    ? ['kB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']
    : ['KiB', 'MiB', 'GiB', 'TiB', 'PiB', 'EiB', 'ZiB', 'YiB'];
  var u = -1;
  do {
    bytes /= thresh;
    ++u;
  } while (bytes >= thresh);
  return bytes.toFixed(1) + ' ' + units[u];
}

export function getTableInfo(hapValue, bank) {
  const { wt, n = 0 } = hapValue;
  let midi = valueToMidi(hapValue, 36);
  let transpose = midi - 36; // C3 is middle C;
  const index = getSoundIndex(n, bank.length);
  const tableUrl = bank[index];
  const label = `${wt}:${index}`;
  return { transpose, tableUrl, index, midi, label };
}

const loadBuffer = (url, ac, wt, n = 0) => {
  const label = wt ? `table "${wt}:${n}"` : 'table';
  url = url.replace('#', '%23');
  if (!loadCache[url]) {
    logger(`[wavetable] load ${label}..`, 'load-table', { url });
    const timestamp = Date.now();
    loadCache[url] = fetch(url)
      .then((res) => res.arrayBuffer())
      .then(async (res) => {
        const took = Date.now() - timestamp;
        const size = humanFileSize(res.byteLength);
        logger(`[wavetable] load ${label}... done! loaded ${size} in ${took}ms`, 'loaded-table', { url });
        const decoded = await ac.decodeAudioData(res);
        return decoded;
      });
  }
  return loadCache[url];
};

function githubPath(base, subpath = '') {
  if (!base.startsWith('github:')) {
    throw new Error('expected "github:" at the start of pseudoUrl');
  }
  let [_, path] = base.split('github:');
  path = path.endsWith('/') ? path.slice(0, -1) : path;
  if (path.split('/').length === 2) {
    // assume main as default branch if none set
    path += '/main';
  }
  return `https://raw.githubusercontent.com/${path}/${subpath}`;
}

const _processTables = (json, baseUrl, frameLen) => {
  return Object.entries(json).forEach(([key, value]) => {
    if (typeof value === 'string') {
      value = [value];
    }
    if (typeof value !== 'object') {
      throw new Error('wrong json format for ' + key);
    }
    baseUrl = value._base || baseUrl;
    if (baseUrl.startsWith('github:')) {
      baseUrl = githubPath(baseUrl, '');
    }
    value = value.map((v) => baseUrl + v);
    registerSound(key, (t, hapValue, onended) => onTriggerSynth(t, hapValue, onended, value, frameLen), {
      type: 'wavetable',
      tables: value,
      baseUrl,
      frameLen,
    });
  });
};

/**
 * Loads a collection of wavetables to use with `wt`
 *
 * @name tables
 */
export const tables = async (url, frameLen, json) => {
  if (json !== undefined) return _processTables(json, url, frameLen);
  if (url.startsWith('github:')) {
    url = githubPath(url, 'strudel.json');
  }
  if (url.startsWith('local:')) {
    url = `http://localhost:5432`;
  }
  if (typeof fetch !== 'function') {
    // not a browser
    return;
  }
  const base = url.split('/').slice(0, -1).join('/');
  if (typeof fetch === 'undefined') {
    // skip fetch when in node / testing
    return;
  }
  return fetch(url)
    .then((res) => res.json())
    .then((json) => _processTables(json, url, frameLen))
    .catch((error) => {
      console.error(error);
      throw new Error(`error loading "${url}"`);
    });
};

async function onTriggerSynth(t, value, onended, bank, frameLen) {
  const { s, n = 0, duration } = value;
  const ac = getAudioContext();
  const [attack, decay, sustain, release] = getADSRValues([value.attack, value.decay, value.sustain, value.release]);
  let { wtWarpMode } = value;
  if (typeof wtWarpMode === 'string') {
    wtWarpMode = WarpMode[wtWarpMode.toUpperCase()] ?? WarpMode.NONE;
  }
  const frequency = getFrequencyFromValue(value);
  const { tableUrl, label } = getTableInfo(value, bank);
  const payload = await loadWavetableFrames(tableUrl, label, frameLen);
  const holdEnd = t + duration;
  const envEnd = holdEnd + release;
  const source = getWorklet(
    ac,
    'wavetable-oscillator-processor',
    {
      begin: t,
      end: envEnd,
      frequency,
      detune: value.detune,
      position: value.wtPos,
      warp: value.wtWarp,
      warpMode: wtWarpMode,
      voices: value.unison,
      spread: value.spread,
      phaserand: value.wtPhaseRand,
    },
    { outputChannelCount: [2] },
  );
  source.port.postMessage({ type: 'tables', payload });
  if (ac.currentTime > t) {
    logger(`[wavetable] still loading sound "${s}:${n}"`, 'highlight');
    return;
  }
  const vibratoOscillator = getVibratoOscillator(source.detune, value, t);
  const envGain = ac.createGain();
  const node = source.connect(envGain);
  getParamADSR(node.gain, attack, decay, sustain, release, 0, 1, t, holdEnd, 'linear');
  getPitchEnvelope(source.detune, value, t, holdEnd);
  const handle = { node, source };
  const timeoutNode = webAudioTimeout(
    ac,
    () => {
      source.disconnect();
      destroyAudioWorkletNode(source);
      vibratoOscillator?.stop();
      node.disconnect();
      onended();
    },
    t,
    envEnd,
  );
  handle.stop = (time) => {
    timeoutNode.stop(time);
  };
  return handle;
}
