import {
  processSampleMap,
  registerSamplesPrefix,
  registerSound,
  onTriggerSample,
  getAudioContext,
  loadBuffer,
} from '@strudel/webaudio';

// Tauri v2 API - lazy loaded
let _readDir, _readBinaryFile, _writeTextFile, _readTextFile, _exists, _BaseDirectory;
let _fsInitialized = false;
let _isTauri = false;

async function ensureFsAPIs() {
  if (_fsInitialized) return _isTauri;

  _fsInitialized = true;

  if (typeof window === 'undefined') {
    _isTauri = false;
    return false;
  }

  try {
    const fs = await import('@tauri-apps/plugin-fs');
    _readDir = fs.readDir;
    _readBinaryFile = fs.readFile;
    _writeTextFile = fs.writeTextFile;
    _readTextFile = fs.readTextFile;
    _exists = fs.exists;
    _BaseDirectory = fs.BaseDirectory;
    _isTauri = true;
    return true;
  } catch (e) {
    console.log('Tauri fs not available:', e);
    _isTauri = false;
    return false;
  }
}

// Export wrapper functions that ensure APIs are loaded
export async function readDir(path, options) {
  await ensureFsAPIs();
  if (!_readDir) throw new Error('readDir not available');
  return _readDir(path, options);
}

export async function readBinaryFile(path, options) {
  await ensureFsAPIs();
  if (!_readBinaryFile) throw new Error('readBinaryFile not available');
  return _readBinaryFile(path, options);
}

export async function writeTextFile(path, contents, options) {
  await ensureFsAPIs();
  if (!_writeTextFile) throw new Error('writeTextFile not available');
  return _writeTextFile(path, contents, options);
}

export async function readTextFile(path, options) {
  await ensureFsAPIs();
  if (!_readTextFile) throw new Error('readTextFile not available');
  return _readTextFile(path, options);
}

export async function exists(path, options) {
  await ensureFsAPIs();
  if (!_exists) throw new Error('exists not available');
  return _exists(path, options);
}

export async function getBaseDirectory() {
  await ensureFsAPIs();
  return _BaseDirectory;
}

export async function getDir() {
  const BaseDirectory = await getBaseDirectory();
  return BaseDirectory?.Audio;
}

const prefix = '~/music/';

async function hasStrudelJson(subpath) {
  const dir = await getDir();
  return exists(subpath + '/strudel.json', { baseDir: dir });
}

async function loadStrudelJson(subpath) {
  const dir = await getDir();
  const contents = await readTextFile(subpath + '/strudel.json', { baseDir: dir });
  const sampleMap = JSON.parse(contents);
  processSampleMap(sampleMap, (key, bank) => {
    registerSound(key, (t, hapValue, onended) => onTriggerSample(t, hapValue, onended, bank, fileResolver(subpath)), {
      type: 'sample',
      samples: bank,
      fileSystem: true,
      tag: 'local',
    });
  });
}

async function writeStrudelJson(subpath) {
  const dir = await getDir();
  const children = await readDir(subpath, { baseDir: dir, recursive: true });
  const name = subpath.split('/').slice(-1)[0];
  const tree = { name, children };

  let samples = {};
  let count = 0;
  walkFileTree(tree, (entry, parent) => {
    if (['wav', 'mp3'].includes(entry.name.split('.').slice(-1)[0])) {
      samples[parent.name] = samples[parent.name] || [];
      count += 1;
      samples[parent.name].push(entry.subpath.slice(1).concat([entry.name]).join('/'));
    }
  });
  const json = JSON.stringify(samples, null, 2);
  const filepath = subpath + '/strudel.json';
  await writeTextFile(filepath, json, { baseDir: dir });
  console.log(`wrote strudel.json with ${count} samples to ${subpath}!`);
}

registerSamplesPrefix(prefix, async (path) => {
  const subpath = path.replace(prefix, '');
  const hasJson = await hasStrudelJson(subpath);
  if (!hasJson) {
    await writeStrudelJson(subpath);
  }
  return loadStrudelJson(subpath);
});

export const walkFileTree = (node, fn) => {
  if (!Array.isArray(node?.children)) {
    return;
  }
  for (const entry of node.children) {
    entry.subpath = (node.subpath || []).concat([node.name]);
    fn(entry, node);
    if (entry.children) {
      walkFileTree(entry, fn);
    }
  }
};

export const isAudioFile = (filename) =>
  ['wav', 'mp3', 'flac', 'ogg', 'm4a', 'aac'].includes(filename.split('.').slice(-1)[0]);

function uint8ArrayToDataURL(uint8Array) {
  const blob = new Blob([uint8Array], { type: 'audio/*' });
  const dataURL = URL.createObjectURL(blob);
  return dataURL;
}

const loadCache = {}; // caches local urls to data urls
export async function resolveFileURL(url) {
  if (loadCache[url]) {
    return loadCache[url];
  }
  const dir = await getDir();
  loadCache[url] = (async () => {
    const contents = await readBinaryFile(url, { baseDir: dir });
    return uint8ArrayToDataURL(contents);
  })();
  return loadCache[url];
}

const fileResolver = (subpath) => (url) => resolveFileURL(subpath.endsWith('/') ? subpath + url : subpath + '/' + url);

export async function playFile(path) {
  const url = await resolveFileURL(path);
  const ac = getAudioContext();
  const bufferSource = ac.createBufferSource();
  bufferSource.buffer = await loadBuffer(url, ac);
  bufferSource.connect(ac.destination);
  bufferSource.start(ac.currentTime);
}
