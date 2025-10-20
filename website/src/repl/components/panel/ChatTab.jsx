import { useState, useEffect, useRef } from 'react';
import cx from '@src/cx.mjs';
import { Textbox } from '../textbox/Textbox';
import { Cog6ToothIcon, TrashIcon, PlusIcon, ArrowPathIcon } from '@heroicons/react/16/solid';

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

export function ChatTab({ context }) {
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [provider, setProvider] = useState('claude-sonnet-4-5-20250929');
  const [apiKey, setApiKey] = useState('');
  const [docsLoaded, setDocsLoaded] = useState(false);
  const [configLoaded, setConfigLoaded] = useState(false);
  const messagesEndRef = useRef(null);
  const codeRef = useRef('');
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
          fetch('/examples.json')
        ]);

        const docsJson = await docsResponse.json();
        const examplesJson = await examplesResponse.json();

        // Send both to backend
        await invoke('load_strudel_docs', {
          docsJson: JSON.stringify(docsJson),
          examplesJson: JSON.stringify(examplesJson)
        });

        setDocsLoaded(true);
        console.log('✅ Strudel docs and examples loaded into chat agent');
      } catch (error) {
        console.error('❌ Failed to load Strudel docs:', error);
        // Try to continue without docs
        setDocsLoaded(true);
      }
    };

    loadDocs();
  }, [docsLoaded]);

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
          console.log('💻 Code context updated -', liveCode.length, 'chars',
                     prevLength === 0 ? '(initial)' : `(+${liveCode.length - prevLength})`);
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

      const assistantMessage = {
        role: 'assistant',
        content: response,
        timestamp: Date.now(),
      };

      setMessages((prev) => [...prev, assistantMessage]);
    } catch (error) {
      console.error('Failed to send message:', error);
      const errorMessage = {
        role: 'assistant',
        content: `Error: ${error}. Make sure you've set your API key in the settings.`,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSaveSettings = async () => {
    if (!TAURI) return;

    const invoke = await getInvoke();
    if (!invoke) return;

    try {
      await invoke('set_chat_config', {
        provider,
        apiKey: apiKey.trim() ? apiKey : null
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
    window.dispatchEvent(new CustomEvent('insert-code', {
      detail: { code: codeToInsert, mode }
    }));
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

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex justify-between items-center p-2 border-b border-lineHighlight">
        <div className="text-sm text-foreground">
          <span className="font-bold">AI Assistant</span>
          {docsLoaded && <span className="ml-2 text-xs opacity-50">(docs loaded)</span>}
          {configLoaded && apiKey && <span className="ml-2 text-xs opacity-50">✓ configured</span>}
          {currentCode && <span className="ml-2 text-xs text-cyan-400">💻 {currentCode.length} chars tracked</span>}
        </div>
        <div className="flex gap-2">
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

      {/* Messages */}
      <div className="flex-grow overflow-y-auto p-4 space-y-4">
        {messages.length === 0 && (
          <div className="text-center text-foreground opacity-50 py-8">
            <p className="mb-2">Ask me anything about Strudel!</p>
            <p className="text-xs">I can help you create music patterns, explain functions, and debug code.</p>
            <div className="mt-4 text-left max-w-md mx-auto text-xs">
              <p className="font-bold mb-1">💡 Pro tips:</p>
              <ul className="list-disc list-inside space-y-1">
                <li>Type <code className="bg-lineBackground px-1">/search function_name</code> to search docs</li>
                <li>Your code is automatically tracked every 2 seconds</li>
                <li>I use {provider.includes('claude') ? 'Claude' : provider.includes('gpt') || provider.includes('o') ? 'OpenAI' : 'Gemini'} {provider.includes('4.5') || provider.includes('5') || provider.includes('o3') || provider.includes('o4') ? 'with extended reasoning' : ''}</li>
              </ul>
            </div>
          </div>
        )}

        {messages.map((msg, idx) => (
          <MessageBubble
            key={idx}
            message={msg}
            onInsertCode={handleInsertCode}
          />
        ))}

        {isLoading && (
          <div className="flex justify-start">
            <div className="px-4 py-2 rounded-lg bg-lineHighlight text-foreground animate-pulse">
              Thinking...
            </div>
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
                : 'bg-[var(--cyan-400)] text-background hover:bg-[var(--cyan-500)]'
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
  const hasCodeBlock = message.content.includes('```');

  const renderContent = () => {
    if (!hasCodeBlock) {
      return <p className="whitespace-pre-wrap">{message.content}</p>;
    }

    // Split content by code blocks
    const parts = message.content.split(/(```(?:javascript)?\n[\s\S]*?```)/);

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
      return <p key={idx} className="whitespace-pre-wrap">{part}</p>;
    });
  };

  return (
    <div className={cx('flex', isUser ? 'justify-end' : 'justify-start')}>
      <div
        className={cx(
          'max-w-[80%] px-4 py-2 rounded-lg',
          isUser
            ? 'bg-[var(--cyan-400)] text-background'
            : 'bg-lineHighlight text-foreground'
        )}
      >
        {renderContent()}
      </div>
    </div>
  );
}
