import React, { useState, useEffect } from 'react';

/**
 * Example Browser Component
 *
 * A component for browsing and managing Strudel examples
 * with filtering, search, and preview capabilities.
 */

export const ExampleBrowser = () => {
  const [examples, setExamples] = useState([]);
  const [filteredExamples, setFilteredExamples] = useState([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [selectedExample, setSelectedExample] = useState(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadExamples();
  }, []);

  useEffect(() => {
    filterExamples();
  }, [examples, searchTerm, selectedCategory]);

  const loadExamples = async () => {
    try {
      const response = await fetch('/examples.json');
      const data = await response.json();
      setExamples(data.examples || []);
    } catch (error) {
      console.error('Failed to load examples:', error);
    } finally {
      setLoading(false);
    }
  };

  const filterExamples = () => {
    let filtered = examples;

    // Filter by category
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((ex) => ex.category === selectedCategory);
    }

    // Filter by search term
    if (searchTerm) {
      const term = searchTerm.toLowerCase();
      filtered = filtered.filter(
        (ex) =>
          ex.name.toLowerCase().includes(term) ||
          ex.description.toLowerCase().includes(term) ||
          ex.code.toLowerCase().includes(term),
      );
    }

    setFilteredExamples(filtered);
  };

  const getCategories = () => {
    const categories = new Set(examples.map((ex) => ex.category));
    return Array.from(categories).sort();
  };

  const copyToClipboard = (code) => {
    navigator.clipboard.writeText(code);
    // You could add a toast notification here
  };

  const runExample = (code) => {
    // This would integrate with your Strudel REPL
    console.log('Running example:', code);
    // You could emit an event or call a parent function here
  };

  if (loading) {
    return (
      <div className="example-browser">
        <div className="loading">Loading examples...</div>
      </div>
    );
  }

  return (
    <div className="example-browser">
      <div className="browser-header">
        <h2>Strudel Examples Browser</h2>
        <div className="controls">
          <input
            type="text"
            placeholder="Search examples..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="search-input"
          />
          <select
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value)}
            className="category-select"
          >
            <option value="all">All Categories</option>
            {getCategories().map((category) => (
              <option key={category} value={category}>
                {category.charAt(0).toUpperCase() + category.slice(1)}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="browser-content">
        <div className="examples-list">
          <div className="examples-count">{filteredExamples.length} examples found</div>

          {filteredExamples.map((example, index) => (
            <div
              key={index}
              className={`example-card ${selectedExample === example ? 'selected' : ''}`}
              onClick={() => setSelectedExample(example)}
            >
              <div className="example-header">
                <h3>{example.name}</h3>
                <span className="category-badge">{example.category}</span>
              </div>
              <p className="example-description">{example.description}</p>
              <div className="example-actions">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    copyToClipboard(example.code);
                  }}
                  className="action-btn copy-btn"
                >
                  Copy
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    runExample(example.code);
                  }}
                  className="action-btn run-btn"
                >
                  Run
                </button>
              </div>
            </div>
          ))}
        </div>

        {selectedExample && (
          <div className="example-detail">
            <div className="detail-header">
              <h3>{selectedExample.name}</h3>
              <button onClick={() => setSelectedExample(null)} className="close-btn">
                ×
              </button>
            </div>
            <div className="detail-content">
              <div className="detail-section">
                <h4>Description</h4>
                <p>{selectedExample.description}</p>
              </div>

              <div className="detail-section">
                <h4>Code</h4>
                <pre className="code-preview">
                  <code>{selectedExample.code}</code>
                </pre>
                <div className="code-actions">
                  <button onClick={() => copyToClipboard(selectedExample.code)} className="action-btn copy-btn">
                    Copy Code
                  </button>
                  <button onClick={() => runExample(selectedExample.code)} className="action-btn run-btn">
                    Run in REPL
                  </button>
                </div>
              </div>

              {selectedExample.author && (
                <div className="detail-section">
                  <h4>Author</h4>
                  <p>{selectedExample.author}</p>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      <style jsx>{`
        .example-browser {
          display: flex;
          flex-direction: column;
          height: 100%;
          background: #1a1a1a;
          color: #ffffff;
        }

        .browser-header {
          padding: 1rem;
          border-bottom: 1px solid #333;
        }

        .browser-header h2 {
          margin: 0 0 1rem 0;
          color: #4caf50;
        }

        .controls {
          display: flex;
          gap: 1rem;
        }

        .search-input,
        .category-select {
          padding: 0.5rem;
          border: 1px solid #555;
          border-radius: 4px;
          background: #2a2a2a;
          color: #ffffff;
        }

        .search-input {
          flex: 1;
        }

        .browser-content {
          display: flex;
          flex: 1;
          overflow: hidden;
        }

        .examples-list {
          flex: 1;
          overflow-y: auto;
          padding: 1rem;
        }

        .examples-count {
          margin-bottom: 1rem;
          color: #888;
          font-size: 0.9rem;
        }

        .example-card {
          background: #2a2a2a;
          border: 1px solid #333;
          border-radius: 8px;
          padding: 1rem;
          margin-bottom: 1rem;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .example-card:hover {
          border-color: #4caf50;
          transform: translateY(-2px);
        }

        .example-card.selected {
          border-color: #4caf50;
          background: #2d4a2d;
        }

        .example-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 0.5rem;
        }

        .example-header h3 {
          margin: 0;
          color: #4caf50;
        }

        .category-badge {
          background: #555;
          padding: 0.25rem 0.5rem;
          border-radius: 12px;
          font-size: 0.8rem;
          text-transform: uppercase;
        }

        .example-description {
          margin: 0 0 1rem 0;
          color: #ccc;
          line-height: 1.4;
        }

        .example-actions {
          display: flex;
          gap: 0.5rem;
        }

        .action-btn {
          padding: 0.5rem 1rem;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-size: 0.9rem;
          transition: background-color 0.2s ease;
        }

        .copy-btn {
          background: #2196f3;
          color: white;
        }

        .copy-btn:hover {
          background: #1976d2;
        }

        .run-btn {
          background: #4caf50;
          color: white;
        }

        .run-btn:hover {
          background: #45a049;
        }

        .example-detail {
          width: 400px;
          background: #2a2a2a;
          border-left: 1px solid #333;
          display: flex;
          flex-direction: column;
        }

        .detail-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 1rem;
          border-bottom: 1px solid #333;
        }

        .detail-header h3 {
          margin: 0;
          color: #4caf50;
        }

        .close-btn {
          background: none;
          border: none;
          color: #888;
          font-size: 1.5rem;
          cursor: pointer;
          padding: 0;
          width: 30px;
          height: 30px;
          display: flex;
          align-items: center;
          justify-content: center;
        }

        .close-btn:hover {
          color: #fff;
        }

        .detail-content {
          flex: 1;
          overflow-y: auto;
          padding: 1rem;
        }

        .detail-section {
          margin-bottom: 1.5rem;
        }

        .detail-section h4 {
          margin: 0 0 0.5rem 0;
          color: #4caf50;
          font-size: 1rem;
        }

        .code-preview {
          background: #1a1a1a;
          border: 1px solid #333;
          border-radius: 4px;
          padding: 1rem;
          overflow-x: auto;
          font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
          font-size: 0.9rem;
          line-height: 1.4;
          margin-bottom: 1rem;
        }

        .code-actions {
          display: flex;
          gap: 0.5rem;
        }

        .loading {
          display: flex;
          align-items: center;
          justify-content: center;
          height: 200px;
          color: #888;
        }
      `}</style>
    </div>
  );
};

export default ExampleBrowser;
