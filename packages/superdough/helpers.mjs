import { getAudioContext } from './audioContext.mjs';
import { clamp, ffloor, nanFallback } from './util.mjs';
import { getNoiseBuffer } from './noise.mjs';

export const noises = ['pink', 'white', 'brown', 'crackle'];

export function gainNode(value) {
  const node = getAudioContext().createGain();
  node.gain.value = value;
  return node;
}

const getSlope = (y1, y2, x1, x2) => {
  const denom = x2 - x1;
  if (denom === 0) {
    return 0;
  }
  return (y2 - y1) / (x2 - x1);
};

export function getWorklet(ac, processor, params, config) {
  const node = new AudioWorkletNode(ac, processor, config);
  Object.entries(params).forEach(([key, value]) => {
    node.parameters.get(key).value = value;
  });
  return node;
}

export const getParamADSR = (
  param,
  attack,
  decay,
  sustain,
  release,
  min,
  max,
  begin,
  end,
  //exponential works better for frequency modulations (such as filter cutoff) due to human ear perception
  curve = 'exponential',
) => {
  attack = nanFallback(attack);
  decay = nanFallback(decay);
  sustain = nanFallback(sustain);
  release = nanFallback(release);
  const ramp = curve === 'exponential' ? 'exponentialRampToValueAtTime' : 'linearRampToValueAtTime';
  if (curve === 'exponential') {
    min = min === 0 ? 0.001 : min;
    max = max === 0 ? 0.001 : max;
  }
  const range = max - min;
  const peak = max;
  const sustainVal = min + sustain * range;
  const duration = end - begin;

  const envValAtTime = (time) => {
    let val;
    if (attack > time) {
      let slope = getSlope(min, peak, 0, attack);
      val = time * slope + (min > peak ? min : 0);
    } else {
      val = (time - attack) * getSlope(peak, sustainVal, 0, decay) + peak;
    }
    if (curve === 'exponential') {
      val = val || 0.001;
    }
    return val;
  };

  param.setValueAtTime(min, begin);
  if (attack > duration) {
    //attack
    param[ramp](envValAtTime(duration), end);
  } else if (attack + decay > duration) {
    //attack
    param[ramp](envValAtTime(attack), begin + attack);
    //decay
    param[ramp](envValAtTime(duration), end);
  } else {
    //attack
    param[ramp](envValAtTime(attack), begin + attack);
    //decay
    param[ramp](envValAtTime(attack + decay), begin + attack + decay);
    //sustain
    param.setValueAtTime(sustainVal, end);
  }
  //release
  param[ramp](min, end + release);
};

export function getCompressor(ac, threshold, ratio, knee, attack, release) {
  const options = {
    threshold: threshold ?? -3,
    ratio: ratio ?? 10,
    knee: knee ?? 10,
    attack: attack ?? 0.005,
    release: release ?? 0.05,
  };
  return new DynamicsCompressorNode(ac, options);
}

// changes the default values of the envelope based on what parameters the user has defined
// so it behaves more like you would expect/familiar as other synthesis tools
// ex: sound(val).decay(val) will behave as a decay only envelope. sound(val).attack(val).decay(val) will behave like an "ad" env, etc.

export const getADSRValues = (params, curve = 'linear', defaultValues) => {
  const envmin = curve === 'exponential' ? 0.001 : 0.001;
  const releaseMin = 0.01;
  const envmax = 1;
  const [a, d, s, r] = params;
  if (a == null && d == null && s == null && r == null) {
    return defaultValues ?? [envmin, envmin, envmax, releaseMin];
  }
  const sustain = s != null ? s : (a != null && d == null) || (a == null && d == null) ? envmax : envmin;
  return [Math.max(a ?? 0, envmin), Math.max(d ?? 0, envmin), Math.min(sustain, envmax), Math.max(r ?? 0, releaseMin)];
};

export function createFilter(context, type, frequency, Q, att, dec, sus, rel, fenv, start, end, fanchor, model, drive) {
  const curve = 'exponential';
  const [attack, decay, sustain, release] = getADSRValues([att, dec, sus, rel], curve, [0.005, 0.14, 0, 0.1]);
  let filter;
  let frequencyParam;
  if (model === 'ladder') {
    filter = getWorklet(context, 'ladder-processor', { frequency, q: Q, drive });
    frequencyParam = filter.parameters.get('frequency');
  } else {
    filter = context.createBiquadFilter();
    filter.type = type;
    filter.Q.value = Q;
    filter.frequency.value = frequency;
    frequencyParam = filter.frequency;
  }

  // envelope is active when any of these values is set
  const hasEnvelope = att ?? dec ?? sus ?? rel ?? fenv;
  // Apply ADSR to filter frequency
  if (hasEnvelope !== undefined) {
    fenv = nanFallback(fenv, 1, true);
    fanchor = nanFallback(fanchor, 0, true);
    const fenvAbs = Math.abs(fenv);
    const offset = fenvAbs * fanchor;
    let min = clamp(2 ** -offset * frequency, 0, 20000);
    let max = clamp(2 ** (fenvAbs - offset) * frequency, 0, 20000);
    if (fenv < 0) [min, max] = [max, min];
    getParamADSR(frequencyParam, attack, decay, sustain, release, min, max, start, end, curve);
    return filter;
  }
  return filter;
}

// stays 1 until .5, then fades out
let wetfade = (d) => (d < 0.5 ? 1 : 1 - (d - 0.5) / 0.5);

// mix together dry and wet nodes. 0 = only dry 1 = only wet
// still not too sure about how this could be used more generally...
export function drywet(dry, wet, wetAmount = 0) {
  const ac = getAudioContext();
  if (!wetAmount) {
    return dry;
  }
  let dry_gain = ac.createGain();
  let wet_gain = ac.createGain();
  dry.connect(dry_gain);
  wet.connect(wet_gain);
  dry_gain.gain.value = wetfade(wetAmount);
  wet_gain.gain.value = wetfade(1 - wetAmount);
  let mix = ac.createGain();
  dry_gain.connect(mix);
  wet_gain.connect(mix);
  return mix;
}

let curves = ['linear', 'exponential'];
export function getPitchEnvelope(param, value, t, holdEnd) {
  // envelope is active when any of these values is set
  const hasEnvelope = value.pattack ?? value.pdecay ?? value.psustain ?? value.prelease ?? value.penv;
  if (!hasEnvelope) {
    return;
  }
  const penv = nanFallback(value.penv, 1, true);
  const curve = curves[value.pcurve ?? 0];
  let [pattack, pdecay, psustain, prelease] = getADSRValues(
    [value.pattack, value.pdecay, value.psustain, value.prelease],
    curve,
    [0.2, 0.001, 1, 0.001],
  );
  let panchor = value.panchor ?? psustain;
  const cents = penv * 100; // penv is in semitones
  const min = 0 - cents * panchor;
  const max = cents - cents * panchor;
  getParamADSR(param, pattack, pdecay, psustain, prelease, min, max, t, holdEnd, curve);
}

export function getVibratoOscillator(param, value, t) {
  const { vibmod = 0.5, vib } = value;
  let vibratoOscillator;
  if (vib > 0) {
    vibratoOscillator = getAudioContext().createOscillator();
    vibratoOscillator.frequency.value = vib;
    const gain = getAudioContext().createGain();
    // Vibmod is the amount of vibrato, in semitones
    gain.gain.value = vibmod * 100;
    vibratoOscillator.connect(gain);
    gain.connect(param);
    vibratoOscillator.start(t);
    return vibratoOscillator;
  }
}
// ConstantSource inherits AudioScheduledSourceNode, which has scheduling abilities
// a bit of a hack, but it works very well :)
export function webAudioTimeout(audioContext, onComplete, startTime, stopTime) {
  const constantNode = new ConstantSourceNode(audioContext);

  // Certain browsers requires audio nodes to be connected in order for their onended events
  // to fire, so we _mute it_ and then connect it to the destination
  const zeroGain = gainNode(0);
  zeroGain.connect(audioContext.destination);
  constantNode.connect(zeroGain);

  // Schedule the `onComplete` callback to occur at `stopTime`
  constantNode.onended = () => {
    // Ensure garbage collection
    try {
      zeroGain.disconnect();
    } catch {
      // pass
    }
    try {
      constantNode.disconnect();
    } catch {
      // pass
    }
    onComplete();
  };
  constantNode.start(startTime);
  constantNode.stop(stopTime);
  return constantNode;
}
const mod = (freq, range = 1, type = 'sine') => {
  const ctx = getAudioContext();
  let osc;
  if (noises.includes(type)) {
    osc = ctx.createBufferSource();
    osc.buffer = getNoiseBuffer(type, 2);
    osc.loop = true;
  } else {
    osc = ctx.createOscillator();
    osc.type = type;
    osc.frequency.value = freq;
  }

  osc.start();
  const g = new GainNode(ctx, { gain: range });
  osc.connect(g); // -range, range
  return { node: g, stop: (t) => osc.stop(t) };
};
const fm = (frequencyparam, harmonicityRatio, modulationIndex, wave = 'sine') => {
  const carrfreq = frequencyparam.value;
  const modfreq = carrfreq * harmonicityRatio;
  const modgain = modfreq * modulationIndex;
  return mod(modfreq, modgain, wave);
};
export function applyFM(param, value, begin) {
  const {
    fmh: fmHarmonicity = 1,
    fmi: fmModulationIndex,
    fmenv: fmEnvelopeType = 'exp',
    fmattack: fmAttack,
    fmdecay: fmDecay,
    fmsustain: fmSustain,
    fmrelease: fmRelease,
    fmvelocity: fmVelocity,
    fmwave: fmWaveform = 'sine',
    duration,
  } = value;
  let modulator;
  let stop = () => {};

  if (fmModulationIndex) {
    const ac = getAudioContext();
    const envGain = ac.createGain();
    const fmmod = fm(param, fmHarmonicity, fmModulationIndex, fmWaveform);

    modulator = fmmod.node;
    stop = fmmod.stop;
    if (![fmAttack, fmDecay, fmSustain, fmRelease, fmVelocity].some((v) => v !== undefined)) {
      // no envelope by default
      modulator.connect(param);
    } else {
      const [attack, decay, sustain, release] = getADSRValues([fmAttack, fmDecay, fmSustain, fmRelease]);
      const holdEnd = begin + duration;
      getParamADSR(
        envGain.gain,
        attack,
        decay,
        sustain,
        release,
        0,
        1,
        begin,
        holdEnd,
        fmEnvelopeType === 'exp' ? 'exponential' : 'linear',
      );
      modulator.connect(envGain);
      envGain.connect(param);
    }
  }
  return { stop };
}

// Saturation curves

const __squash = (x) => x / (1 + x); // [0, inf) to [0, 1)

const _scurve = (x, k) => ((1 + k) * x) / (1 + k * Math.abs(x));
const _soft = (x, k) => Math.tanh(x * (1 + k));
const _hard = (x, k) => clamp((1 + k) * x, -1, 1);
const _sine = (x, k) => Math.sin(x * (1 + k));

const _fold = (x, k) => {
  let y = (1 + k) * x;
  while (y > 1 || y < -1) {
    y = y > 1 ? 2 - y : -2 - y;
  }
  return y;
};

const _sineFold = (x, k) => Math.sin((Math.PI / 2) * _fold(x, k));

const _pow = (x, k) => {
  const t = __squash(k);
  const p = 1 / (1 + 0.5 * t); // tame k
  return _soft(Math.sign(x) * Math.pow(Math.abs(x), p), 0.5 * k);
};

const _cubic = (x, k) => {
  const t = __squash(k);
  const cubic = (x - (t / 3) * x * x * x) / (1 - t / 3); // normalized to go from (-1, 1)
  return _soft(cubic, k);
};

const _diode = (x, k, asym = false) => {
  const g = 1 + k; // gain
  const t = __squash(k);
  const bias = 0.15 * t;
  const pos = _soft(x + bias, k);
  const neg = _soft(asym ? bias : -x + bias, k);
  const y = pos - neg;
  // We divide by the derivative at 0 so that the distortion is roughly
  // the identity map near 0 => small values are preserved and undistorted
  const sech = 1 / Math.cosh(g * bias);
  const sech2 = sech * sech; // derivative of tanh is sech^2
  const denom = Math.max(1e-8, (asym ? 1 : 2) * g * sech2); // g from chain rule; 2 if both pos/neg have x
  return _soft(y / denom, k);
};

const _asym = (x, k) => _diode(x, k, true);

const _chebyshev = (x, k) => {
  const kl = Math.log1p(k);
  let tnm1 = 1;
  let tnm2 = x;
  let tn;
  let y = 0;
  const iterations = 1 + ffloor(Math.min(16 * kl, 255));
  for (let i = 1; i < iterations; i++) {
    if (i < 2) {
      // Already set inital conditions
      y += i == 0 ? tnm1 : tnm2;
      continue;
    }
    tn = 2 * x * tnm1 - tnm2; // https://en.wikipedia.org/wiki/Chebyshev_polynomials#Recurrence_definition
    tnm2 = tnm1;
    tnm1 = tn;
    if (i % 2 === 0) {
      y += tn;
    }
  }
  // Soft clip
  return _soft(y, 0);
};

export const distortionAlgorithms = {
  scurve: _scurve,
  soft: _soft,
  hard: _hard,
  sine: _sine,
  fold: _fold,
  sinefold: _sineFold,
  pow: _pow,
  cubic: _cubic,
  diode: _diode,
  asym: _asym,
  chebyshev: _chebyshev,
};
const _algoNames = Object.freeze(Object.keys(distortionAlgorithms));

export const getDistortionAlgorithm = (algo) => {
  let index = algo;
  if (typeof algo === 'string') {
    index = _algoNames.indexOf(algo);
    if (index === -1) {
      logger(`[superdough] Could not find waveshaping algorithm ${algo}.
        Available options are ${_algoNames.join(', ')}.
        Defaulting to ${_algoNames[0]}.`);
      index = 0;
    }
  }
  const name = _algoNames[index % _algoNames.length]; // allow for wrapping if algo was a number
  const algorithm = distortionAlgorithms[name];
  return { algorithm, index };
};

export const getDistortion = (distort, postgain, algorithm) => {
  const { _algorithm, index } = getDistortionAlgorithm(algorithm);
  return getWorklet(getAudioContext(), 'distort-processor', { distort, postgain, algorithm: index });
};
