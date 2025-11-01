import { useState, useEffect, useRef } from 'react';
import cx from '@src/cx.mjs';
import { Textbox } from '../textbox/Textbox';
import {
  Cog6ToothIcon,
  TrashIcon,
  PlusIcon,
  ArrowPathIcon,
  PlayIcon,
  ForwardIcon,
  XMarkIcon,
  EyeIcon,
  ClockIcon,
} from '@heroicons/react/16/solid';

const TAURI = typeof window !== 'undefined' && window.__TAURI_INTERNALS__;

// Lazy load Tauri APIs
let _invoke;
async function getInvoke() {
  if (!_invoke && TAURI) {
    const tauriCore = await import('@tauri-apps/api/core');
    _invoke = tauriCore.invoke;
  }
  return _invoke;
}

// Queue Panel Component
function QueuePanel({ context }) {
  const [showPreview, setShowPreview] = useState(false);
  const nextChange = context.previewNextChange();

  if (!nextChange) return null;

  const remainingCount = context.changeQueue.length;
  const cyclesWaited = context.cyclesSinceLastChange || 0;
  const waitCycles = nextChange.waitCycles || 0;
  const canApply = cyclesWaited >= waitCycles;

  return (
    <div className="border-b border-purple-500 bg-gradient-to-r from-purple-900/20 to-purple-800/10 p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 text-purple-400 font-bold text-sm">
            <ClockIcon className="w-4 h-4" />
            <span>Queue Active</span>
          </div>
          <div className="text-xs text-foreground opacity-50">
            Step {context.currentStep + 1} • {remainingCount} pending
          </div>
        </div>
        <button
          onClick={() => context.clearQueue()}
          className="text-xs px-2 py-1 text-red-400 hover:bg-red-900/20 rounded flex items-center gap-1"
          title="Clear Queue"
        >
          <XMarkIcon className="w-3 h-3" />
          Clear
        </button>
      </div>

      {/* Next Change Preview */}
      <div className="bg-background/50 rounded p-2 mb-2">
        <div className="flex items-center justify-between mb-1">
          <span className="text-xs font-bold text-purple-300">Next: {nextChange.description || 'Pattern Change'}</span>
          <button
            onClick={() => setShowPreview(!showPreview)}
            className="text-xs px-2 py-0.5 bg-purple-500/20 text-purple-300 hover:bg-purple-500/30 rounded flex items-center gap-1"
          >
            <EyeIcon className="w-3 h-3" />
            {showPreview ? 'Hide' : 'Preview'}
          </button>
        </div>
        {showPreview && (
          <pre className="text-xs font-mono text-foreground opacity-70 bg-lineBackground p-2 rounded overflow-x-auto mt-2">
            {nextChange.code}
          </pre>
        )}
      </div>

      {/* Timing Info */}
      {waitCycles > 0 && (
        <div className="text-xs text-foreground opacity-60 mb-2 flex items-center gap-2">
          <ClockIcon className="w-3 h-3" />
          {canApply ? (
            <span className="text-green-400">Ready to apply (waited {cyclesWaited} cycles)</span>
          ) : (
            <span>
              Waiting... ({cyclesWaited}/{waitCycles} cycles)
            </span>
          )}
        </div>
      )}

      {/* Action Buttons */}
      <div className="flex gap-2">
        <button
          onClick={() => context.applyNextChange()}
          disabled={!canApply && waitCycles > 0}
          className={cx(
            'flex-1 px-3 py-1.5 rounded text-xs font-bold flex items-center justify-center gap-1',
            canApply || waitCycles === 0
              ? 'bg-purple-500 text-white hover:bg-purple-600'
              : 'bg-lineHighlight text-foreground opacity-50 cursor-not-allowed',
          )}
        >
          <PlayIcon className="w-3 h-3" />
          Apply Next
        </button>
        <button
          onClick={() => context.skipNextChange()}
          className="px-3 py-1.5 bg-orange-500/20 text-orange-300 hover:bg-orange-500/30 rounded text-xs flex items-center gap-1"
        >
          <ForwardIcon className="w-3 h-3" />
          Skip
        </button>
        <button
          onClick={() => context.applyAllChanges()}
          className="px-3 py-1.5 bg-green-500/20 text-green-300 hover:bg-green-500/30 rounded text-xs flex items-center gap-1"
        >
          <ArrowPathIcon className="w-3 h-3" />
          Apply All
        </button>
      </div>

      {/* Queue List Preview */}
      {remainingCount > 1 && (
        <div className="mt-2 text-xs text-foreground opacity-50">
          <details>
            <summary className="cursor-pointer hover:opacity-70">View all {remainingCount} queued changes</summary>
            <div className="mt-2 space-y-1 max-h-32 overflow-y-auto">
              {context.changeQueue.map((change, idx) => (
                <div key={idx} className="flex items-center gap-2 p-1 bg-lineBackground rounded">
                  <span className="text-purple-400 font-mono">{idx + 1}.</span>
                  <span>{change.description || `Change ${idx + 1}`}</span>
                  {change.waitCycles > 0 && (
                    <span className="ml-auto text-xs opacity-50">({change.waitCycles} cycles)</span>
                  )}
                </div>
              ))}
            </div>
          </details>
        </div>
      )}
    </div>
  );
}

// Parse queue items from agent messages using simple markdown format
function parseAndQueueChanges(content, addToQueue) {
  try {
    const queueItems = [];

    // Pattern 1: Look for queue blocks with emojis (🎬 or 📋)
    // Format: 🎬 QUEUE: Description (mode, wait N cycles)
    //         ```javascript
    //         code here
    //         ```
    const queueBlockRegex = /(?:🎬|📋)\s*QUEUE:\s*([^\n]+)\n```(?:javascript|js)?\n([\s\S]+?)```/g;
    let match;

    while ((match = queueBlockRegex.exec(content)) !== null) {
      const header = match[1].trim();
      const code = match[2].trim();

      // Parse header: "Description (replace, wait 4 cycles)"
      const modeMatch = header.match(/\((replace|append)/i);
      const waitMatch = header.match(/wait\s+(\d+)\s+cycles?/i);
      const descMatch = header.match(/^([^(]+)/);

      const item = {
        description: descMatch ? descMatch[1].trim() : 'Pattern change',
        code: code,
        mode: modeMatch ? modeMatch[1].toLowerCase() : 'replace',
        waitCycles: waitMatch ? parseInt(waitMatch[1]) : 0,
        autoEvaluate: true,
      };

      queueItems.push(item);
      console.log('📋 Parsed queue item:', item.description);
    }

    // Pattern 2: Fallback to old JSON format for backwards compatibility
    if (queueItems.length === 0) {
      const jsonMatch = content.match(/```json\s*(\{[\s\S]*?"queue"[\s\S]*?\})\s*```/);
      if (jsonMatch) {
        const queueData = JSON.parse(jsonMatch[1]);
        if (queueData.queue && Array.isArray(queueData.queue)) {
          queueItems.push(...queueData.queue);
          console.log('📋 Parsed JSON queue with', queueItems.length, 'items');
        }
      }
    }

    // Add all parsed items to queue
    if (queueItems.length > 0) {
      console.log('✅ Adding', queueItems.length, 'items to queue');
      addToQueue(queueItems);
    }
  } catch (error) {
    console.error('Failed to parse queue:', error);
  }
}

export function ChatTab({ context }) {
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [streamingMessage, setStreamingMessage] = useState(null);
  const [showSettings, setShowSettings] = useState(false);
  const [provider, setProvider] = useState('claude-sonnet-4-5-20250929');
  const [apiKey, setApiKey] = useState('');
  const [allowLiveEdit, setAllowLiveEdit] = useState(false);
  const [docsLoaded, setDocsLoaded] = useState(false);
  const [configLoaded, setConfigLoaded] = useState(false);
  const messagesEndRef = useRef(null);
  const codeRef = useRef('');
  const streamingBufferRef = useRef('');
  const hadStreamingRef = useRef(false);
  const [currentCode, setCurrentCode] = useState(''); // Track current code for re-rendering

  // Auto-scroll to bottom when new messages arrive
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  // Load saved config on mount
  useEffect(() => {
    if (!TAURI || configLoaded) return;

    const loadConfig = async () => {
      const invoke = await getInvoke();
      if (!invoke) return;

      try {
        const config = await invoke('get_chat_config');
        if (config.provider) {
          setProvider(config.provider);
        }
        if (config.api_key) {
          setApiKey(config.api_key);
        }
        if (typeof config.live_edit_enabled === 'boolean') {
          setAllowLiveEdit(config.live_edit_enabled);
        }
        setConfigLoaded(true);
        console.log('✅ Chat config loaded from store');
      } catch (error) {
        console.error('Failed to load chat config:', error);
        setConfigLoaded(true);
      }
    };

    loadConfig();
  }, [configLoaded]);

  // Load chat history on mount
  useEffect(() => {
    if (!TAURI) return;

    const loadHistory = async () => {
      const invoke = await getInvoke();
      if (!invoke) return;

      try {
        const history = await invoke('get_chat_history');
        setMessages(history);
      } catch (error) {
        console.error('Failed to load chat history:', error);
      }
    };

    loadHistory();
  }, []);

  // Load Strudel docs and examples on mount
  useEffect(() => {
    if (!TAURI || docsLoaded) return;

    const loadDocs = async () => {
      const invoke = await getInvoke();
      if (!invoke) return;

      try {
        // Fetch doc.json and examples.json in parallel
        const [docsResponse, examplesResponse] = await Promise.all([
          fetch('/doc.json'),
          fetch('/examples-safe.json').catch(() => null), // Safe examples, optional
        ]);

        // Check if docs response is ok before parsing
        if (!docsResponse.ok) {
          throw new Error(`Failed to fetch docs: ${docsResponse.status} ${docsResponse.statusText}`);
        }

        const docsJson = await docsResponse.json();
        const examplesJson = examplesResponse ? await examplesResponse.json() : { examples: [], creativity_tips: [] };

        // Send both to backend
        await invoke('load_strudel_docs', {
          docsJson: JSON.stringify(docsJson),
          examplesJson: JSON.stringify(examplesJson),
        });

        setDocsLoaded(true);
        console.log('✅ Strudel docs and examples loaded into chat agent');
      } catch (error) {
        console.error('❌ Failed to load Strudel docs:', error);

        // Fallback: Create minimal docs from Context7 data
        const fallbackDocs = {
          docs: [
            {
              name: 'note',
              description: 'Create musical note patterns',
              examples: ['note("c3 eb3 g3")', 'note("c3 [e3 g3]*2")'],
            },
            {
              name: 's',
              description: 'Select sounds/samples by name',
              examples: ['s("bd hh")', 's("bd sd [- bd] sd")'],
            },
            {
              name: 'sound',
              description: 'Play sounds with optional bank selection',
              examples: ['sound("bd sd")', 'sound("bd sd").bank("RolandTR909")'],
            },
            {
              name: 'stack',
              description: 'Layer patterns vertically',
              examples: ['stack(note("c2"), s("bd"))'],
            },
            {
              name: 'cat',
              description: 'Concatenate patterns horizontally',
              examples: ['cat("g3,b3,e4", "a3,c3,e4")'],
            },
            {
              name: 'fast',
              description: 'Speed up patterns',
              examples: ['fast(2)', 'fast("<1 2 4>")'],
            },
            {
              name: 'slow',
              description: 'Slow down patterns',
              examples: ['slow(2)', 'slow("<1 2 4>")'],
            },
            {
              name: 'scale',
              description: 'Apply musical scales to notes',
              examples: ['scale("C:major")', 'scale("C4:minor")'],
            },
            {
              name: 'gain',
              description: 'Control volume/amplitude',
              examples: ['gain(0.5)', 'gain("<0.3 0.7>")'],
            },
            {
              name: 'pan',
              description: 'Control stereo panning',
              examples: ['pan(0.5)', 'pan("<0 1>")'],
            },
            {
              name: 'lpf',
              description: 'Low-pass filter',
              examples: ['lpf(800)', 'lpf("<400 1200>")'],
            },
            {
              name: 'hpf',
              description: 'High-pass filter',
              examples: ['hpf(200)', 'hpf("<100 500>")'],
            },
            {
              name: 'room',
              description: 'Reverb effect',
              examples: ['room(0.5)', 'room("<0.2 0.8>")'],
            },
            {
              name: 'delay',
              description: 'Delay effect',
              examples: ['delay(0.5)', 'delay("<0.2 0.8>")'],
            },
            {
              name: 'sometimes',
              description: 'Apply function with probability',
              examples: ['sometimes(x => x.fast(2))', 'sometimes(x => x.rev())'],
            },
            {
              name: 'often',
              description: 'Apply function often (80% chance)',
              examples: ['often(x => x.fast(2))'],
            },
            {
              name: 'rarely',
              description: 'Apply function rarely (20% chance)',
              examples: ['rarely(x => x.fast(2))'],
            },
          ],
        };

        const fallbackExamples = {
          examples: [
            {
              name: 'Basic Beat',
              code: 's("bd hh sd hh")',
              description: 'Simple four-on-the-floor beat',
            },
            {
              name: 'Melodic Pattern',
              code: 'note("c3 eb3 g3 bb3").scale("C:minor")',
              description: 'Minor scale melody',
            },
            {
              name: 'Layered Pattern',
              code: 'stack(s("bd sd"), note("c2 eb2 g2"))',
              description: 'Drums and bass together',
            },
          ],
          creativity_tips: [
            'Use mini-notation for quick pattern creation',
            'Layer different elements with stack()',
            'Add variation with sometimes() and often()',
            'Experiment with scales and chords',
            'Use effects like reverb and delay for atmosphere',
          ],
        };

        // Send fallback docs to backend
        await invoke('load_strudel_docs', {
          docsJson: JSON.stringify(fallbackDocs),
          examplesJson: JSON.stringify(fallbackExamples),
        });

        console.log('✅ Fallback Strudel docs loaded');
        setDocsLoaded(true);
      }
    };

    loadDocs();
  }, [docsLoaded]);

  // Listen for streaming updates from Tauri backend
  useEffect(() => {
    if (!TAURI) return undefined;

    let unlisten;

    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen('chat-stream', ({ payload }) => {
          if (!payload || !payload.event) return;
          const eventType = payload.event;
          switch (eventType) {
            case 'start':
              hadStreamingRef.current = true;
              streamingBufferRef.current = '';
              setStreamingMessage({
                content: '',
                reasoning: [],
                provider: payload.provider,
                model: payload.model,
              });
              setIsLoading(true);
              break;
            case 'delta': {
              streamingBufferRef.current += payload.content || '';
              setStreamingMessage((prev) => (prev ? { ...prev, content: streamingBufferRef.current } : prev));
              break;
            }
            case 'reasoning': {
              const thought = payload.content;
              if (!thought) break;
              setStreamingMessage((prev) =>
                prev
                  ? {
                      ...prev,
                      reasoning: [...(prev.reasoning || []), thought],
                    }
                  : prev,
              );
              break;
            }
            case 'done': {
              const finalContent = payload.content || streamingBufferRef.current || '';
              streamingBufferRef.current = '';
              if (finalContent) {
                const assistantMessage = {
                  role: 'assistant',
                  content: finalContent,
                  timestamp: Date.now(),
                };
                if (payload.usage) {
                  assistantMessage.usage = payload.usage;
                }
                setMessages((prev) => [...prev, assistantMessage]);

                // Check for queue JSON in the message
                if (context?.queueEnabled && context?.addToQueue) {
                  parseAndQueueChanges(finalContent, context.addToQueue);
                }
              }
              setStreamingMessage(null);
              setIsLoading(false);
              break;
            }
            case 'error': {
              const errorText = payload.content || 'Streaming error. Check your model settings and try again.';
              setMessages((prev) => [
                ...prev,
                {
                  role: 'assistant',
                  content: errorText,
                  timestamp: Date.now(),
                },
              ]);
              streamingBufferRef.current = '';
              setStreamingMessage(null);
              setIsLoading(false);
              break;
            }
            default:
              break;
          }
        });
      } catch (error) {
        console.error('Failed to attach chat-stream listener', error);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // Listen for live edit events and pipe them into the REPL
  useEffect(() => {
    if (!TAURI) return undefined;

    let unlisten;

    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen('chat-live-edit', ({ payload }) => {
          if (!payload || !payload.mode || typeof payload.code !== 'string') return;

          window.dispatchEvent(
            new CustomEvent('insert-code', {
              detail: { code: payload.code, mode: payload.mode },
            }),
          );

          setMessages((prev) => [
            ...prev,
            {
              role: 'assistant',
              content: `✏️ Applied ${payload.mode} live edit (${payload.code.length} chars)`,
              timestamp: Date.now(),
            },
          ]);
        });
      } catch (error) {
        console.error('Failed to attach chat-live-edit listener', error);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // Listen for queue edit events and add to queue
  // Store latest context in a ref to avoid re-subscribing
  const contextRef = useRef(context);
  useEffect(() => {
    contextRef.current = context;
  }, [context]);

  // Set up event listener ONCE (empty dependency array)
  useEffect(() => {
    if (!TAURI) return undefined;

    let unlisten;

    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen('chat-queue-edit', ({ payload }) => {
          if (!payload || !payload.mode || typeof payload.code !== 'string') return;

          console.log('📋 Received queue edit:', payload.description);

          // Add to queue - queue size limit is enforced in useReplContext.addToQueue
          if (contextRef.current?.addToQueue) {
            const queueItem = {
              description: payload.description || 'Pattern change',
              code: payload.code,
              mode: payload.mode,
              waitCycles: payload.wait_cycles || 0,
              autoEvaluate: true,
            };
            contextRef.current.addToQueue(queueItem);

            // Show feedback message
            setMessages((prev) => [
              ...prev,
              {
                role: 'assistant',
                content: `🎬 Queued: ${queueItem.description} (wait ${queueItem.waitCycles} cycles)`,
                timestamp: Date.now(),
              },
            ]);
          }
        });
      } catch (error) {
        console.error('Failed to attach chat-queue-edit listener', error);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []); // Empty array = set up once on mount, clean up on unmount

  // Poll editor for live code updates
  useEffect(() => {
    if (!TAURI || !context?.editorRef) return;

    const updateCodeContext = async () => {
      const invoke = await getInvoke();
      if (!invoke) return;

      // Get live code directly from editor
      const liveCode = context.editorRef.current?.code || '';

      // Only update if code has changed
      if (liveCode && codeRef.current !== liveCode) {
        const prevLength = codeRef.current.length;
        codeRef.current = liveCode;
        setCurrentCode(liveCode);

        try {
          await invoke('set_code_context', { code: liveCode });
          console.log(
            '💻 Code context updated -',
            liveCode.length,
            'chars',
            prevLength === 0 ? '(initial)' : `(+${liveCode.length - prevLength})`,
          );
        } catch (error) {
          console.error('❌ Failed to update code context:', error);
        }
      }
    };

    // Update immediately
    updateCodeContext();

    // Poll every 2 seconds for code changes
    const interval = setInterval(updateCodeContext, 2000);

    return () => clearInterval(interval);
  }, [context?.editorRef]);

  const handleSendMessage = async () => {
    if (!input.trim() || isLoading || !TAURI) return;

    const invoke = await getInvoke();
    if (!invoke) return;
    streamingBufferRef.current = '';
    setStreamingMessage(null);
    hadStreamingRef.current = false;

    const userMessage = {
      role: 'user',
      content: input,
      timestamp: Date.now(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);

    try {
      const response = await invoke('send_chat_message', { message: input });

      if (!hadStreamingRef.current) {
        const assistantMessage = {
          role: 'assistant',
          content: response,
          timestamp: Date.now(),
        };
        setMessages((prev) => [...prev, assistantMessage]);
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      const errorMessage = {
        role: 'assistant',
        content: `Error: ${error}. Make sure you've set your API key in the settings.`,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      if (!hadStreamingRef.current) {
        setIsLoading(false);
      }
      hadStreamingRef.current = false;
      streamingBufferRef.current = '';
    }
  };

  const handleSaveSettings = async () => {
    if (!TAURI) return;

    const invoke = await getInvoke();
    if (!invoke) return;

    try {
      await invoke('set_chat_config', {
        provider,
        apiKey: apiKey.trim() ? apiKey : null,
        liveEditEnabled: allowLiveEdit,
      });
      setShowSettings(false);
      alert('Settings saved! You can now start chatting.');
    } catch (error) {
      console.error('Failed to save settings:', error);
      alert(`Failed to save settings: ${error}`);
    }
  };

  const handleClearHistory = async () => {
    if (!TAURI) return;
    if (!confirm('Clear all chat history?')) return;

    const invoke = await getInvoke();
    if (!invoke) return;

    try {
      await invoke('clear_chat_history');
      setMessages([]);
    } catch (error) {
      console.error('Failed to clear history:', error);
    }
  };

  const handleInsertCode = (code, mode = 'append') => {
    // Extract code from markdown code blocks
    const codeMatch = code.match(/```(?:javascript)?\n([\s\S]*?)```/);
    const codeToInsert = codeMatch ? codeMatch[1] : code;

    // Emit custom event to insert code into editor
    window.dispatchEvent(
      new CustomEvent('insert-code', {
        detail: { code: codeToInsert, mode },
      }),
    );
  };

  if (!TAURI) {
    return (
      <div className="flex items-center justify-center h-full p-4 text-foreground opacity-50">
        Chat is only available in Strudel Desktop
      </div>
    );
  }

  if (showSettings) {
    return (
      <div className="flex flex-col h-full p-4 gap-4">
        <h3 className="text-lg font-bold text-foreground">Chat Settings</h3>

        <div className="flex flex-col gap-2">
          <label className="text-sm text-foreground">Provider / Model</label>
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value)}
            className="px-3 py-2 bg-background text-foreground border border-lineHighlight rounded"
          >
            <optgroup label="OpenAI (Recommended for Speed)">
              <option value="gpt-5">GPT-5 ⭐ Latest & Best</option>
              <option value="gpt-5-pro">GPT-5 Pro (Extended Reasoning)</option>
              <option value="o4-mini">o4-mini (Fast Reasoning)</option>
              <option value="o3">o3 (Advanced Reasoning)</option>
              <option value="o3-pro">o3-pro (Max Reasoning)</option>
              <option value="gpt-4o">GPT-4o</option>
              <option value="gpt-4o-mini">GPT-4o Mini</option>
            </optgroup>
            <optgroup label="Anthropic Claude (Best for Code)">
              <option value="claude-sonnet-4-5-20250929">Claude Sonnet 4.5 ⭐ Best for Coding</option>
              <option value="claude-opus-4-1-20250805">Claude Opus 4.1 (Complex Tasks)</option>
              <option value="claude-haiku-4-5-20251001">Claude Haiku 4.5 (Fast)</option>
              <option value="claude-3-5-sonnet-20241022">Claude 3.5 Sonnet (Legacy)</option>
              <option value="claude-3-5-haiku-20241022">Claude 3.5 Haiku (Legacy)</option>
            </optgroup>
            <optgroup label="Google Gemini">
              <option value="gemini-2.5-pro-exp">Gemini 2.5 Pro (Experimental)</option>
              <option value="gemini-2.5-flash">Gemini 2.5 Flash (Thinking)</option>
              <option value="gemini-2.0-flash-thinking-exp">Gemini 2.0 Flash Thinking</option>
              <option value="gemini-2.0-flash-exp">Gemini 2.0 Flash</option>
            </optgroup>
            <optgroup label="Local Models">
              <option value="ollama:llama3.2">Ollama: Llama 3.2 (Free, Local)</option>
            </optgroup>
          </select>
        </div>

        <div className="flex flex-col gap-2">
          <label className="text-sm text-foreground">API Key (optional for Ollama)</label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-..."
            className="px-3 py-2 bg-background text-foreground border border-lineHighlight rounded"
          />
          <p className="text-xs text-foreground opacity-50">
            For OpenAI, Anthropic, or Google models. Not needed for Ollama.
          </p>
          <p className="text-xs text-foreground opacity-50 mt-2">
            ⚡ Models marked with ⭐ or containing "Pro", "o3", "o4", "Thinking" support extended reasoning.
          </p>
        </div>

        <div className="flex flex-col gap-2">
          <label className="text-sm text-foreground">Live edit autopilot</label>
          <label className="flex items-center gap-2 text-sm text-foreground">
            <input
              type="checkbox"
              checked={allowLiveEdit}
              onChange={(e) => setAllowLiveEdit(e.target.checked)}
              className="w-4 h-4"
            />
            Allow the assistant to apply code changes directly while streaming
          </label>
          <p className="text-xs text-foreground opacity-50">
            When enabled, the assistant can append or replace your REPL code automatically. You can still undo manually.
          </p>
        </div>

        <div className="flex gap-2">
          <button
            onClick={handleSaveSettings}
            className="px-4 py-2 bg-[var(--cyan-400)] text-background rounded hover:bg-[var(--cyan-500)]"
          >
            Save Settings
          </button>
          <button
            onClick={() => setShowSettings(false)}
            className="px-4 py-2 bg-lineHighlight text-foreground rounded hover:bg-lineBackground"
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  // Queue toggle button in header
  const handleToggleQueue = () => {
    if (context?.setQueueEnabled) {
      context.setQueueEnabled((prev) => !prev);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex justify-between items-center p-2 border-b border-lineHighlight">
        <div className="text-sm text-foreground">
          <span className="font-bold">AI Assistant</span>
          {docsLoaded && <span className="ml-2 text-xs opacity-50">(docs loaded)</span>}
          {configLoaded && apiKey && <span className="ml-2 text-xs opacity-50">✓ configured</span>}
          {currentCode && <span className="ml-2 text-xs text-cyan-400">💻 {currentCode.length} chars tracked</span>}
          {allowLiveEdit && <span className="ml-2 text-xs text-lime-400">🛠 live edits on</span>}
          {context?.queueEnabled && <span className="ml-2 text-xs text-purple-400">🎬 queue mode</span>}
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleToggleQueue}
            className={cx(
              'text-xs px-2 py-1 rounded',
              context?.queueEnabled ? 'bg-purple-500 text-white' : 'text-foreground hover:bg-lineHighlight',
            )}
            title="Toggle Queue Mode"
          >
            🎬
          </button>
          <button
            onClick={() => setShowSettings(true)}
            className="text-xs px-2 py-1 text-foreground hover:bg-lineHighlight rounded"
            title="Settings"
          >
            <Cog6ToothIcon className="w-4 h-4" />
          </button>
          <button
            onClick={handleClearHistory}
            className="text-xs px-2 py-1 text-foreground hover:bg-lineHighlight rounded"
            title="Clear History"
          >
            <TrashIcon className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Queue UI */}
      {context?.queueEnabled && context?.changeQueue && context.changeQueue.length > 0 && (
        <QueuePanel context={context} />
      )}

      {/* Messages */}
      <div className="flex-grow overflow-y-auto p-4 space-y-4">
        {messages.length === 0 && (
          <div className="text-center text-foreground opacity-50 py-8">
            <p className="mb-2">Ask me anything about Strudel!</p>
            <p className="text-xs">I can help you create music patterns, explain functions, and debug code.</p>
            <div className="mt-4 text-left max-w-md mx-auto text-xs">
              <p className="font-bold mb-1">💡 Pro tips:</p>
              <ul className="list-disc list-inside space-y-1">
                <li>
                  Type <code className="bg-lineBackground px-1">/search function_name</code> to search docs
                </li>
                <li>Your code is automatically tracked every 2 seconds</li>
                <li>
                  I use{' '}
                  {provider.includes('claude')
                    ? 'Claude'
                    : provider.includes('gpt') || provider.includes('o')
                      ? 'OpenAI'
                      : 'Gemini'}{' '}
                  {provider.includes('4.5') ||
                  provider.includes('5') ||
                  provider.includes('o3') ||
                  provider.includes('o4')
                    ? 'with extended reasoning'
                    : ''}
                </li>
              </ul>
            </div>
          </div>
        )}

        {messages.map((msg, idx) => (
          <MessageBubble key={idx} message={msg} onInsertCode={handleInsertCode} />
        ))}

        {streamingMessage && (
          <MessageBubble
            message={{
              role: 'assistant',
              content: streamingMessage.content || '…',
              reasoning: streamingMessage.reasoning,
              streaming: true,
            }}
            onInsertCode={handleInsertCode}
          />
        )}

        {isLoading && !streamingMessage && (
          <div className="flex justify-start">
            <div className="px-4 py-2 rounded-lg bg-lineHighlight text-foreground animate-pulse">Thinking...</div>
          </div>
        )}

        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-lineHighlight">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && handleSendMessage()}
            placeholder="Ask about Strudel patterns..."
            disabled={isLoading}
            className="flex-grow px-3 py-2 bg-background text-foreground border border-lineHighlight rounded"
          />
          <button
            onClick={handleSendMessage}
            disabled={isLoading || !input.trim()}
            className={cx(
              'px-4 py-2 rounded',
              isLoading || !input.trim()
                ? 'bg-lineHighlight text-foreground opacity-50 cursor-not-allowed'
                : 'bg-[var(--cyan-400)] text-background hover:bg-[var(--cyan-500)]',
            )}
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}

function MessageBubble({ message, onInsertCode }) {
  const isUser = message.role === 'user';
  const content = message.content || '';
  const hasCodeBlock = content.includes('```');

  const renderContent = () => {
    if (!content) {
      return <p className="whitespace-pre-wrap opacity-70">{message.streaming ? '…' : ''}</p>;
    }

    if (!hasCodeBlock) {
      return <p className="whitespace-pre-wrap">{content}</p>;
    }

    // Split content by code blocks
    const parts = content.split(/(```(?:javascript)?\n[\s\S]*?```)/);

    return parts.map((part, idx) => {
      const codeMatch = part.match(/```(?:javascript)?\n([\s\S]*?)```/);
      if (codeMatch) {
        const code = codeMatch[1];
        return (
          <div key={idx} className="my-2">
            <pre className="bg-background p-3 rounded overflow-x-auto text-xs">
              <code>{code}</code>
            </pre>
            <div className="mt-2 flex gap-2">
              <button
                onClick={() => onInsertCode(code, 'append')}
                className="text-xs px-3 py-1 bg-[var(--cyan-400)] text-background rounded hover:bg-[var(--cyan-500)] flex items-center gap-1"
                title="Add this code to the end of your current code"
              >
                <PlusIcon className="w-3 h-3" />
                Append
              </button>
              <button
                onClick={() => onInsertCode(code, 'replace')}
                className="text-xs px-3 py-1 bg-orange-500 text-white rounded hover:bg-orange-600 flex items-center gap-1"
                title="Replace all your code with this"
              >
                <ArrowPathIcon className="w-3 h-3" />
                Replace All
              </button>
            </div>
          </div>
        );
      }
      return (
        <p key={idx} className="whitespace-pre-wrap">
          {part}
        </p>
      );
    });
  };

  return (
    <div className={cx('flex', isUser ? 'justify-end' : 'justify-start')}>
      <div
        className={cx(
          'max-w-[80%] px-4 py-2 rounded-lg',
          isUser ? 'bg-[var(--cyan-400)] text-background' : 'bg-lineHighlight text-foreground',
          message.streaming && !isUser && 'opacity-80',
        )}
      >
        {renderContent()}
        {message.reasoning && message.reasoning.length > 0 && (
          <div className="text-xs opacity-60 italic mt-2">{message.reasoning.slice(-3).join(' ')}</div>
        )}
      </div>
    </div>
  );
}
