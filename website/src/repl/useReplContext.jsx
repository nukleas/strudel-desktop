/*
Repl.jsx - <short description TODO>
Copyright (C) 2022 Strudel contributors - see <https://codeberg.org/uzu/strudel/src/branch/main/repl/src/App.js>
This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details. You should have received a copy of the GNU Affero General Public License along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

import { code2hash, getPerformanceTimeSeconds, logger, silence } from '@strudel/core';
import { getDrawContext } from '@strudel/draw';
import { transpiler } from '@strudel/transpiler';
import {
  getAudioContextCurrentTime,
  webaudioOutput,
  resetGlobalEffects,
  resetLoadedSounds,
  initAudioOnFirstClick,
  resetDefaults,
} from '@strudel/webaudio';
import { setVersionDefaultsFrom } from './util.mjs';
import { StrudelMirror, defaultSettings } from '@strudel/codemirror';
import { clearHydra } from '@strudel/hydra';
import { useCallback, useEffect, useRef, useState } from 'react';
import { parseBoolean, settingsMap, useSettings } from '../settings.mjs';
import {
  setActivePattern,
  setLatestCode,
  createPatternID,
  userPattern,
  getViewingPatternData,
  setViewingPatternData,
} from '../user_pattern_utils.mjs';
import { superdirtOutput } from '@strudel/osc/superdirtoutput';
import { audioEngineTargets } from '../settings.mjs';
import { useStore } from '@nanostores/react';
import { prebake } from './prebake.mjs';
import { getRandomTune, initCode, loadModules, shareCode } from './util.mjs';
import './Repl.css';
import { setInterval, clearInterval } from 'worker-timers';
import { getMetadata } from '../metadata_parser';
// Register validation handler for Tauri
import '../validation';

const { latestCode, maxPolyphony, audioDeviceName, multiChannelOrbits } = settingsMap.get();
let modulesLoading, presets, drawContext, clearCanvas, audioReady;

if (typeof window !== 'undefined') {
  audioReady = initAudioOnFirstClick({
    maxPolyphony,
    audioDeviceName,
    multiChannelOrbits: parseBoolean(multiChannelOrbits),
  });
  modulesLoading = loadModules();
  presets = prebake();
  drawContext = getDrawContext();
  clearCanvas = () => drawContext.clearRect(0, 0, drawContext.canvas.height, drawContext.canvas.width);
}

async function getModule(name) {
  if (!modulesLoading) {
    return;
  }
  const modules = await modulesLoading;
  return modules.find((m) => m.packageName === name);
}

const initialCode = `// LOADING`;

export function useReplContext() {
  const { isSyncEnabled, audioEngineTarget, autoEvalEnabled } = useSettings();
  const shouldUseWebaudio = audioEngineTarget !== audioEngineTargets.osc;
  const defaultOutput = shouldUseWebaudio ? webaudioOutput : superdirtOutput;
  const getTime = shouldUseWebaudio ? getAudioContextCurrentTime : getPerformanceTimeSeconds;

  const init = useCallback(() => {
    const drawTime = [-2, 2];
    const drawContext = getDrawContext();
    const editor = new StrudelMirror({
      sync: isSyncEnabled,
      defaultOutput,
      getTime,
      setInterval,
      clearInterval,
      transpiler,
      autodraw: false,
      root: containerRef.current,
      initialCode,
      pattern: silence,
      drawTime,
      drawContext,
      autoEvalEnabled,
      prebake: async () => Promise.all([modulesLoading, presets]),
      onUpdateState: (state) => {
        setReplState({ ...state });
      },
      onToggle: (playing) => {
        if (!playing) {
          clearHydra();
        }
      },
      beforeEval: () => audioReady,
      afterEval: (all) => {
        const { code } = all;
        //post to iframe parent (like Udels) if it exists...
        window.parent?.postMessage(code);

        setLatestCode(code);
        window.location.hash = '#' + code2hash(code);
        setDocumentTitle(code);
        const viewingPatternData = getViewingPatternData();
        setVersionDefaultsFrom(code);
        const data = { ...viewingPatternData, code };
        let id = data.id;
        const isExamplePattern = viewingPatternData.collection !== userPattern.collection;

        if (isExamplePattern) {
          const codeHasChanged = code !== viewingPatternData.code;
          if (codeHasChanged) {
            // fork example
            const newPattern = userPattern.duplicate(data);
            id = newPattern.id;
            setViewingPatternData(newPattern.data);
          }
        } else {
          id = userPattern.isValidID(id) ? id : createPatternID();
          setViewingPatternData(userPattern.update(id, data).data);
        }
        setActivePattern(id);
      },
      bgFill: false,
    });
    window.strudelMirror = editor;

    // init settings
    initCode().then(async (decoded) => {
      let code, msg;
      if (decoded) {
        code = decoded;
        msg = `I have loaded the code from the URL.`;
      } else if (latestCode) {
        code = latestCode;
        msg = `Your last session has been loaded!`;
      } else {
        /* const { code: randomTune, name } = await getRandomTune();
        code = randomTune; */
        code = '$: s("[bd <hh oh>]*2").bank("tr909").dec(.4)';
        msg = `Default code has been loaded`;
      }
      editor.setCode(code);
      setDocumentTitle(code);
      logger(`Welcome to Strudel! ${msg} Press play or hit ctrl+enter to run it!`, 'highlight');
    });

    editorRef.current = editor;
  }, [autoEvalEnabled]);

  const [replState, setReplState] = useState({});
  const { started, isDirty, error, activeCode, pending } = replState;
  const editorRef = useRef();
  const containerRef = useRef();

  // Listen for code insertion events from chat
  useEffect(() => {
    const handleInsertCode = (event) => {
      if (editorRef.current && event.detail) {
        const { code, mode } = typeof event.detail === 'string'
          ? { code: event.detail, mode: 'append' }
          : event.detail;

        if (mode === 'replace') {
          // Replace all code
          editorRef.current.setCode(code.trim());
        } else {
          // Append to existing code
          const currentCode = editorRef.current.code || '';
          const newCode = currentCode.trim() + '\n\n' + code.trim();
          editorRef.current.setCode(newCode);
        }
      }
    };

    window.addEventListener('insert-code', handleInsertCode);
    return () => window.removeEventListener('insert-code', handleInsertCode);
  }, []);

  // this can be simplified once SettingsTab has been refactored to change codemirrorSettings directly!
  // this will be the case when the main repl is being replaced
  const _settings = useStore(settingsMap, { keys: Object.keys(defaultSettings) });
  useEffect(() => {
    let editorSettings = {};
    Object.keys(defaultSettings).forEach((key) => {
      if (Object.prototype.hasOwnProperty.call(_settings, key)) {
        editorSettings[key] = _settings[key];
      }
    });
    editorRef.current?.updateSettings(editorSettings);
  }, [_settings]);

  //
  // UI Actions
  //

  const setDocumentTitle = (code) => {
    const meta = getMetadata(code);
    document.title = (meta.title ? `${meta.title} - ` : '') + 'Strudel REPL';
  };

  const handleTogglePlay = async () => {
    editorRef.current?.toggle();
  };

  const resetEditor = async () => {
    (await getModule('@strudel/tonal'))?.resetVoicings();
    resetDefaults();
    resetGlobalEffects();
    clearCanvas();
    clearHydra();
    resetLoadedSounds();
    editorRef.current.repl.setCps(0.5);
    await prebake(); // declare default samples
  };

  const handleUpdate = async (patternData, reset = false) => {
    setViewingPatternData(patternData);
    editorRef.current.setCode(patternData.code);
    if (reset) {
      await resetEditor();
      handleEvaluate();
    }
  };

  const handleEvaluate = () => {
    editorRef.current.evaluate();
  };
  const handleShuffle = async () => {
    const patternData = await getRandomTune();
    const code = patternData.code;
    logger(`[repl] ✨ loading random tune "${patternData.id}"`);
    setActivePattern(patternData.id);
    setViewingPatternData(patternData);
    await resetEditor();
    editorRef.current.setCode(code);
    editorRef.current.repl.evaluate(code);
  };

  const handleShare = async () => shareCode(replState.code);

  // Get live code from editor (updates as user types)
  const getLiveCode = () => {
    return editorRef.current?.code || activeCode || '';
  };

  // Queue system state
  const [changeQueue, setChangeQueue] = useState([]);
  const [currentStep, setCurrentStep] = useState(0);
  const [queueEnabled, setQueueEnabled] = useState(false);
  const [cyclesSinceLastChange, setCyclesSinceLastChange] = useState(0);
  const [lastChangeCycle, setLastChangeCycle] = useState(0);

  // Track cycles for timing using Strudel's native cycle counter
  useEffect(() => {
    if (!queueEnabled || !started) return;

    const interval = setInterval(() => {
      if (editorRef.current?.repl?.scheduler) {
        const currentCycle = Math.floor(editorRef.current.repl.scheduler.now());
        const cyclesElapsed = currentCycle - lastChangeCycle;
        setCyclesSinceLastChange(cyclesElapsed);
      }
    }, 100); // Check 10 times per second for accuracy

    return () => clearInterval(interval);
  }, [queueEnabled, started, lastChangeCycle]);

  // Queue actions
  const addToQueue = (items) => {
    const queueItems = Array.isArray(items) ? items : [items];

    // Soft limit: max 5 items recommended (prevents excessive queuing)
    const MAX_QUEUE_SIZE = 5;
    const remainingSlots = MAX_QUEUE_SIZE - changeQueue.length;

    if (remainingSlots <= 0) {
      console.warn(`🎬 Queue full (${MAX_QUEUE_SIZE} items). Cannot add more. Apply or clear existing items first.`);
      return;
    }

    const itemsToAdd = queueItems.slice(0, remainingSlots);

    if (itemsToAdd.length < queueItems.length) {
      console.warn(`🎬 Queue limit: adding ${itemsToAdd.length}/${queueItems.length} items (${MAX_QUEUE_SIZE} max)`);
    }

    setChangeQueue(prev => [...prev, ...itemsToAdd]);

    // Initialize cycle tracking if this is the first item
    if (changeQueue.length === 0 && editorRef.current?.repl?.scheduler) {
      const currentCycle = Math.floor(editorRef.current.repl.scheduler.now());
      setLastChangeCycle(currentCycle);
      setCyclesSinceLastChange(0);
    }
  };

  const applyNextChange = () => {
    if (changeQueue.length === 0) return;

    const nextChange = changeQueue[0];
    if (!nextChange) return;

    // Apply the change
    if (nextChange.mode === 'replace') {
      editorRef.current?.setCode(nextChange.code.trim());
    } else {
      const currentCode = editorRef.current?.code || '';
      const newCode = currentCode.trim() + '\n\n' + nextChange.code.trim();
      editorRef.current?.setCode(newCode);
    }

    // Mark as applied and remove from queue
    setChangeQueue(prev => prev.slice(1));
    setCurrentStep(prev => prev + 1);

    // Reset cycle counter using Strudel's current cycle
    if (editorRef.current?.repl?.scheduler) {
      const currentCycle = Math.floor(editorRef.current.repl.scheduler.now());
      setLastChangeCycle(currentCycle);
      setCyclesSinceLastChange(0);
    }

    // Auto-evaluate if requested
    if (nextChange.autoEvaluate !== false) {
      setTimeout(() => handleEvaluate(), 100);
    }
  };

  const skipNextChange = () => {
    setChangeQueue(prev => prev.slice(1));
  };

  const clearQueue = () => {
    setChangeQueue([]);
    setCurrentStep(0);
  };

  const previewNextChange = () => {
    return changeQueue[0] || null;
  };

  const applyAllChanges = () => {
    if (changeQueue.length === 0) return;

    let finalCode = editorRef.current?.code || '';

    changeQueue.forEach(change => {
      if (change.mode === 'replace') {
        finalCode = change.code.trim();
      } else {
        finalCode = finalCode.trim() + '\n\n' + change.code.trim();
      }
    });

    editorRef.current?.setCode(finalCode);
    setChangeQueue([]);
    setCurrentStep(prev => prev + changeQueue.length);

    setTimeout(() => handleEvaluate(), 100);
  };

  // Auto-advance: automatically apply next change when wait cycles elapsed
  useEffect(() => {
    if (!queueEnabled || changeQueue.length === 0) return;

    const nextChange = changeQueue[0];
    const waitCycles = nextChange.waitCycles || 0;

    // Check if we've waited long enough
    if (cyclesSinceLastChange >= waitCycles && waitCycles >= 0) {
      console.log(`🎬 Auto-applying "${nextChange.description}" after ${cyclesSinceLastChange} cycles (waited ${waitCycles})`);
      applyNextChange();
    }
  }, [cyclesSinceLastChange, changeQueue, queueEnabled]);

  const context = {
    started,
    pending,
    isDirty,
    activeCode,
    code: getLiveCode(), // Live code from editor
    getLiveCode, // Function to get latest code
    handleTogglePlay,
    handleUpdate,
    handleShuffle,
    handleShare,
    handleEvaluate,
    init,
    error,
    editorRef,
    containerRef,
    // Queue system
    changeQueue,
    queueEnabled,
    setQueueEnabled,
    addToQueue,
    applyNextChange,
    skipNextChange,
    clearQueue,
    previewNextChange,
    applyAllChanges,
    currentStep,
    cyclesSinceLastChange,
  };
  return context;
}
