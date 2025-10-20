import React, { useState, useEffect } from 'react';
import { useSettings } from '@src/settings.mjs';
import { 
  MusicalNoteIcon, 
  SpeakerWaveIcon, 
  SpeakerXMarkIcon,
  MusicalNoteIcon as NoteIcon,
  SpeakerWaveIcon as SynthIcon,
  CircleStackIcon,
  SparklesIcon,
  ArrowPathIcon,
  BoltIcon,
  DocumentTextIcon,
  WrenchScrewdriverIcon,
  MusicalNoteIcon as ScaleIcon,
  UserIcon,
  MusicalNoteIcon as MelodyIcon,
  MusicalNoteIcon as CompleteIcon,
  StarIcon,
  ComputerDesktopIcon,
  MusicalNoteIcon as JazzIcon,
  SpeakerWaveIcon as ElectronicIcon,
  GlobeAltIcon,
  CloudIcon,
  FireIcon,
  LightBulbIcon,
  ArrowDownTrayIcon,
  PlayIcon,
  ClipboardDocumentIcon
} from '@heroicons/react/16/solid';

// Example categories with icons - showcasing Strudel's capabilities
const exampleCategories = {
  classical: { name: 'Classical', icon: MusicalNoteIcon, description: 'Traditional and contemporary classical compositions' },
  jazz: { name: 'Jazz', icon: JazzIcon, description: 'Jazz fusion and improvisational pieces' },
  techno: { name: 'Techno', icon: ElectronicIcon, description: 'Electronic dance music and techno beats' },
  rock: { name: 'Rock', icon: FireIcon, description: 'Rock and alternative music styles' },
  experimental: { name: 'Experimental', icon: SparklesIcon, description: 'Avant-garde and experimental compositions' },
  ambient: { name: 'Ambient', icon: CloudIcon, description: 'Atmospheric and ambient soundscapes' },
  community: { name: 'Community', icon: UserIcon, description: 'Diverse community-created compositions' },
  all: { name: 'All Songs', icon: CircleStackIcon, description: 'Complete collection of 70+ community songs' },
};

export function ExamplesTab({ context }) {
  const { fontFamily } = useSettings();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState(null);
  const [examples, setExamples] = useState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [selectedExample, setSelectedExample] = useState(null);
  const [showDetails, setShowDetails] = useState(false);

  // Load examples from the API
  useEffect(() => {
    loadExamples();
  }, []);

  const loadExamples = async () => {
    try {
      setLoading(true);
      // Try API endpoint first, fallback to static file
      let response;
      try {
        response = await fetch('/api/examples');
        if (!response.ok) throw new Error('API not available');
      } catch (apiError) {
        // Fallback to static examples.json
        response = await fetch('/examples.json');
      }
      
      if (!response.ok) {
        throw new Error('Failed to load examples');
      }
      
      const data = await response.json();
      setExamples(data.examples || data);
    } catch (err) {
      console.error('Error loading examples:', err);
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  const handleInsertExample = (example) => {
    if (context?.editorRef?.current) {
      const editor = context.editorRef.current;
      const cursor = editor.getCursorLocation();
      const currentCode = editor.code || '';

      // Insert at cursor position
      const beforeCursor = currentCode.substring(0, cursor);
      const afterCursor = currentCode.substring(cursor);
      const newCode = beforeCursor + example.code + afterCursor;

      editor.setCode(newCode);
      // Move cursor to end of inserted example
      editor.setCursorLocation(cursor + example.code.length);
    }
  };

  const handleCopyExample = async (example) => {
    try {
      await navigator.clipboard.writeText(example.code);
      // You could add a toast notification here
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const handleRunExample = (example) => {
    if (context?.editorRef?.current) {
      const editor = context.editorRef.current;
      editor.setCode(example.code);
    }
  };

  const filteredExamples = examples.filter((example) => {
    const matchesSearch =
      !searchQuery ||
      example.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      example.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      example.code.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = !selectedCategory || example.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const groupedExamples = {};
  filteredExamples.forEach((example) => {
    if (!groupedExamples[example.category]) {
      groupedExamples[example.category] = [];
    }
    groupedExamples[example.category].push(example);
  });

  if (loading) {
    return (
      <div className="flex flex-col h-full" style={{ fontFamily }}>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[var(--cyan-400)] mx-auto mb-4"></div>
            <div className="text-[var(--foreground)] opacity-50 font-mono text-sm">Loading examples...</div>
          </div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col h-full" style={{ fontFamily }}>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <div className="text-red-400 mb-2">⚠️ Error loading examples</div>
            <div className="text-[var(--foreground)] opacity-50 font-mono text-sm mb-4">{error}</div>
            <button
              onClick={loadExamples}
              className="px-4 py-2 bg-[var(--cyan-400)] text-[#0f172a] rounded font-mono text-sm hover:bg-[var(--cyan-300)] transition-colors"
            >
              Retry
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full" style={{ fontFamily }}>
      
      <div className="p-3 border-b border-[var(--border-cyan)] bg-[rgba(34,211,238,0.05)]">
        <div className="mb-3">
          <h3 className="text-sm font-mono text-[var(--cyan-400)] mb-1">Community Songs Collection</h3>
          <p className="text-xs text-[var(--foreground)] opacity-60">
            70+ real compositions by eefano • Licensed under CC BY-NC-SA
          </p>
        </div>
        <input
          type="text"
          placeholder="Search 70+ community songs..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full px-3 py-2 bg-[#0f172a] border border-[var(--border-cyan)] rounded-md text-[var(--foreground)] placeholder-[var(--cyan-400)] placeholder-opacity-30 focus:outline-none focus:border-[var(--cyan-400)] font-mono text-sm"
        />
        <div className="flex gap-2 mt-2 flex-wrap">
          <button
            onClick={() => setSelectedCategory(null)}
            className={`px-2 py-1 rounded text-xs font-mono uppercase tracking-wider transition-all ${
              !selectedCategory
                ? 'bg-[var(--cyan-400)] text-[#0f172a]'
                : 'bg-[rgba(34,211,238,0.1)] text-[var(--cyan-400)] hover:bg-[rgba(34,211,238,0.2)]'
            }`}
            title="Show all 70+ community songs"
          >
            All Songs
          </button>
          {Object.entries(exampleCategories).map(([key, category]) => (
            <button
              key={key}
              onClick={() => setSelectedCategory(selectedCategory === key ? null : key)}
              className={`px-2 py-1 rounded text-xs font-mono uppercase tracking-wider transition-all flex items-center gap-1 ${
                selectedCategory === key
                  ? 'bg-[var(--cyan-400)] text-[#0f172a]'
                  : 'bg-[rgba(34,211,238,0.1)] text-[var(--cyan-400)] hover:bg-[rgba(34,211,238,0.2)]'
              }`}
              title={category.description}
            >
              {React.createElement(category.icon, { className: "w-3 h-3" })}
              {category.name}
            </button>
          ))}
        </div>
      </div>

      {/* Examples List */}
      <div className="flex-1 overflow-auto p-3">
        {Object.entries(groupedExamples).length === 0 ? (
          <div className="text-center py-8 text-[var(--foreground)] opacity-50 font-mono text-sm">
            No examples found
          </div>
        ) : (
          Object.entries(groupedExamples).map(([categoryKey, categoryExamples]) => (
            <div key={categoryKey} className="mb-4">
              <h3 className="text-xs font-mono uppercase tracking-widest text-[var(--cyan-400)] mb-2 flex items-center gap-2">
                {React.createElement(exampleCategories[categoryKey]?.icon || MusicalNoteIcon, { className: "w-4 h-4" })}
                <span>{exampleCategories[categoryKey]?.name || categoryKey}</span>
                <span className="text-[var(--foreground)] opacity-30">({categoryExamples.length})</span>
              </h3>
              <div className="space-y-2">
                {categoryExamples.map((example, index) => (
                  <div
                    key={`${example.name}-${index}`}
                    className="w-full p-3 rounded-md bg-[rgba(34,211,238,0.05)] border border-[var(--border-cyan)] hover:bg-[rgba(34,211,238,0.1)] hover:border-[var(--cyan-400)] transition-all group"
                  >
                    <div className="flex justify-between items-start mb-2">
                      <div>
                        <div className="font-medium text-sm text-[var(--cyan-400)]">{example.name}</div>
                        <div className="text-xs text-[var(--foreground)] opacity-50">
                          {example.category} • {example.author} • {example.license}
                        </div>
                      </div>
                      <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button
                          onClick={() => handleInsertExample(example)}
                          className="p-1 hover:bg-[rgba(34,211,238,0.2)] rounded"
                          title="Insert at cursor"
                        >
                          <ArrowDownTrayIcon className="w-3 h-3 text-[var(--cyan-400)]" />
                        </button>
                        <button
                          onClick={() => handleRunExample(example)}
                          className="p-1 hover:bg-[rgba(34,211,238,0.2)] rounded"
                          title="Run this complete song"
                        >
                          <PlayIcon className="w-3 h-3 text-[var(--cyan-400)]" />
                        </button>
                        <button
                          onClick={() => handleCopyExample(example)}
                          className="p-1 hover:bg-[rgba(34,211,238,0.2)] rounded"
                          title="Copy song code"
                        >
                          <ClipboardDocumentIcon className="w-3 h-3 text-[var(--cyan-400)]" />
                        </button>
                      </div>
                    </div>
                    <div className="text-xs text-[var(--foreground)] opacity-60 mb-2">{example.description}</div>
                    <pre className="text-xs font-mono text-[var(--foreground)] opacity-70 bg-[#0f172a] p-2 rounded overflow-x-auto group-hover:opacity-90 transition-opacity">
                      {example.code}
                    </pre>
                    <div className="flex justify-between items-center mt-2 text-xs text-[var(--foreground)] opacity-40">
                      <span>Complete composition</span>
                      <span>Click to explore Strudel's capabilities</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-2 border-t border-[var(--border-cyan)] bg-[rgba(34,211,238,0.05)]">
        <div className="flex justify-between items-center text-xs font-mono text-[var(--foreground)] opacity-50">
          <span>{filteredExamples.length} community songs</span>
          <div className="flex gap-4">
            <span>Insert</span>
            <span>Play</span>
            <span>Copy</span>
          </div>
        </div>
        <div className="text-xs font-mono text-[var(--foreground)] opacity-30 mt-1">
          Discover what's possible with Strudel • Open source community compositions
        </div>
      </div>
    </div>
  );
}