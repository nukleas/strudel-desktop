import { getAudioContext, registerSound } from './index.mjs';
import { getSoundIndex, valueToMidi } from './util.mjs';
import {
  destroyAudioWorkletNode,
  getADSRValues,
  getFrequencyFromValue,
  getLfo,
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
  const buf = await loadBuffer(url, label);
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

export function getTableInfo(hapValue, tableUrls) {
  const { s, n = 0 } = hapValue;
  let midi = valueToMidi(hapValue, 36);
  let transpose = midi - 36; // C3 is middle C;
  const index = getSoundIndex(n, tableUrls.length);
  const tableUrl = tableUrls[index];
  const label = `${s}:${index}`;
  return { transpose, tableUrl, index, midi, label };
}

// Extract the sample rate of a .wav file
function parseWavSampleRate(arrBuf) {
  const dv = new DataView(arrBuf);
  // Header is "RIFF<chunk size (4 bytes)>WAVE", so 12 bytes
  let p = 12;
  // Look through chunks for the format header
  // (they will always have an 8 byte header (id and size) followed by a payload)
  while (p + 8 <= dv.byteLength) {
    // Parse id
    const id = String.fromCharCode(dv.getUint8(p), dv.getUint8(p + 1), dv.getUint8(p + 2), dv.getUint8(p + 3));
    // Parse chunk size
    const size = dv.getUint32(p + 4, true);
    if (id === 'fmt ') {
      // The format chunk contains the sample rate after
      // 8 bytes of header, 2 bytes of format tag, 2 bytes of num channels
      // (for a total of 12)
      return dv.getUint32(p + 12, true);
    }
    // Advance to next chunk
    p += 8 + size + (size & 1);
  }
  return null;
}

async function decodeAtNativeRate(arr) {
  const sr = parseWavSampleRate(arr) || 44100;
  const tempAC = new OfflineAudioContext(1, 1, sr);
  return await tempAC.decodeAudioData(arr);
}

const loadBuffer = (url, label) => {
  url = url.replace('#', '%23');
  if (!loadCache[url]) {
    logger(`[wavetable] load table ${label}..`, 'load-table', { url });
    const timestamp = Date.now();
    loadCache[url] = fetch(url)
      .then((res) => res.arrayBuffer())
      .then(async (res) => {
        const took = Date.now() - timestamp;
        const size = humanFileSize(res.byteLength);
        logger(`[wavetable] load table ${label}... done! loaded ${size} in ${took}ms`, 'loaded-table', { url });
        const decoded = await decodeAtNativeRate(res);
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

const _processTables = (json, baseUrl, frameLen, options = {}) => {
  baseUrl = json._base || baseUrl;
  return Object.entries(json).forEach(([key, tables]) => {
    if (key === '_base') return false;
    if (typeof tables === 'string') {
      tables = [tables];
    }
    if (typeof tables !== 'object') {
      throw new Error('wrong json format for ' + key);
    }
    let resolvedUrl = baseUrl;
    if (resolvedUrl.startsWith('github:')) {
      resolvedUrl = githubPath(resolvedUrl, '');
    }
    tables = tables
      .map((t) => resolvedUrl + t)
      .filter((t) => {
        if (!t.toLowerCase().endsWith('.wav')) {
          logger(`[wavetable] skipping ${t} -- wavetables must be ".wav" format`);
          return false;
        }
        return true;
      });
    if (tables.length) {
      const { prebake, tag } = options;
      registerSound(key, (t, hapValue, onended) => onTriggerSynth(t, hapValue, onended, tables, frameLen), {
        type: 'wavetable',
        tables,
        baseUrl,
        frameLen,
        prebake,
        tag,
      });
    }
  });
};

/**
 * Loads a collection of wavetables to use with `s`
 *
 * @name tables
 */
export const tables = async (url, frameLen, json, options = {}) => {
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
  if (typeof fetch === 'undefined') {
    // skip fetch when in node / testing
    return;
  }
  return fetch(url)
    .then((res) => res.json())
    .then((json) => _processTables(json, url, frameLen, options))
    .catch((error) => {
      console.error(error);
      throw new Error(`error loading "${url}"`);
    });
};

async function onTriggerSynth(t, value, onended, tables, frameLen) {
  const { s, n = 0, duration } = value;
  const ac = getAudioContext();
  const [attack, decay, sustain, release] = getADSRValues([value.attack, value.decay, value.sustain, value.release]);
  let { wtWarpMode } = value;
  if (typeof wtWarpMode === 'string') {
    wtWarpMode = WarpMode[wtWarpMode.toUpperCase()] ?? WarpMode.NONE;
  }
  const frequency = getFrequencyFromValue(value);
  const { tableUrl, label } = getTableInfo(value, tables);
  const payload = await loadWavetableFrames(tableUrl, label, frameLen);
  const holdEnd = t + duration;
  const endWithRelease = holdEnd + release;
  const envEnd = endWithRelease + 0.01;
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
  const posADSRParams = [value.wtPosAttack, value.wtPosDecay, value.wtPosSustain, value.wtPosRelease];
  const warpADSRParams = [value.wtPosAttack, value.wtPosDecay, value.wtPosSustain, value.wtPosRelease];
  const wtParams = source.parameters;
  const positionParam = wtParams.get('position');
  const warpParam = wtParams.get('warp');
  if (posADSRParams.some((p) => p !== undefined)) {
    const [pAttack, pDecay, pSustain, pRelease] = getADSRValues(posADSRParams);
    getParamADSR(positionParam, pAttack, pDecay, pSustain, pRelease, 0, 1, t, holdEnd, 'linear');
  } else {
    const posLFO = getLfo(ac, t, endWithRelease, {
      frequency: value.wtPosRate,
      depth: value.wtPosDepth,
      shape: value.wtPosShape,
      skew: value.wtPosSkew,
      dcoffset: value.wtPosDCOffset ?? 0,
    });
    posLFO.connect(positionParam);
  }
  if (posADSRParams.some((p) => p !== undefined)) {
    const [wAttack, wDecay, wSustain, wRelease] = getADSRValues(warpADSRParams);
    getParamADSR(warpParam, wAttack, wDecay, wSustain, wRelease, 0, 1, t, holdEnd, 'linear');
  } else {
    const warpLFO = getLfo(ac, t, endWithRelease, {
      frequency: value.wtWarpRate,
      depth: value.wtWarpDepth,
      shape: value.wtWarpShape,
      skew: value.wtWarpSkew,
      dcoffset: value.wtWarpDCOffset ?? 0,
    });
    warpLFO.connect(warpParam);
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
      warpLFO.disconnect();
      posLFO.disconnect();
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
