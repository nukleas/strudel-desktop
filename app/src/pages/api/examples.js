/**
 * Examples API Endpoint
 *
 * Provides REST API endpoints for accessing Strudel examples
 * with filtering, search, and metadata capabilities.
 */

import fs from 'fs/promises';
import path from 'path';

export async function GET({ url, request }) {
  try {
    const examplesPath = path.join(process.cwd(), 'public/examples.json');
    const examplesData = await fs.readFile(examplesPath, 'utf-8');
    const examples = JSON.parse(examplesData);

    // Parse query parameters
    const searchParams = url.searchParams;
    const category = searchParams.get('category');
    const search = searchParams.get('search');
    const limit = parseInt(searchParams.get('limit')) || 50;
    const offset = parseInt(searchParams.get('offset')) || 0;
    const format = searchParams.get('format') || 'json';

    let filteredExamples = examples.examples || [];

    // Filter by category
    if (category && category !== 'all') {
      filteredExamples = filteredExamples.filter((ex) => ex.category === category);
    }

    // Filter by search term
    if (search) {
      const searchTerm = search.toLowerCase();
      filteredExamples = filteredExamples.filter(
        (ex) =>
          ex.name.toLowerCase().includes(searchTerm) ||
          ex.description.toLowerCase().includes(searchTerm) ||
          ex.code.toLowerCase().includes(searchTerm),
      );
    }

    // Apply pagination
    const total = filteredExamples.length;
    const paginatedExamples = filteredExamples.slice(offset, offset + limit);

    // Prepare response data
    const responseData = {
      examples: paginatedExamples,
      pagination: {
        total,
        limit,
        offset,
        hasMore: offset + limit < total,
      },
      metadata: {
        categories: [...new Set(examples.examples?.map((ex) => ex.category) || [])],
        totalExamples: examples.examples?.length || 0,
        lastUpdated: new Date().toISOString(),
      },
    };

    // Return different formats based on request
    if (format === 'csv') {
      const csv = convertToCSV(paginatedExamples);
      return new Response(csv, {
        headers: {
          'Content-Type': 'text/csv',
          'Content-Disposition': 'attachment; filename="strudel-examples.csv"',
        },
      });
    }

    if (format === 'txt') {
      const txt = convertToText(paginatedExamples);
      return new Response(txt, {
        headers: {
          'Content-Type': 'text/plain',
        },
      });
    }

    // Default JSON response
    return new Response(JSON.stringify(responseData, null, 2), {
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'GET, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type',
      },
    });
  } catch (error) {
    console.error('Error serving examples:', error);
    return new Response(JSON.stringify({ error: 'Failed to load examples' }), {
      status: 500,
      headers: {
        'Content-Type': 'application/json',
      },
    });
  }
}

export async function OPTIONS() {
  return new Response(null, {
    status: 200,
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type',
    },
  });
}

function convertToCSV(examples) {
  const headers = ['name', 'category', 'description', 'code', 'author'];
  const csvRows = [headers.join(',')];

  examples.forEach((example) => {
    const row = headers.map((header) => {
      const value = example[header] || '';
      // Escape quotes and wrap in quotes if contains comma
      return value.includes(',') || value.includes('"') ? `"${value.replace(/"/g, '""')}"` : value;
    });
    csvRows.push(row.join(','));
  });

  return csvRows.join('\n');
}

function convertToText(examples) {
  return examples
    .map(
      (example) =>
        `${example.name}\nCategory: ${example.category}\nDescription: ${example.description}\n\nCode:\n${example.code}\n\n${'='.repeat(50)}\n`,
    )
    .join('\n');
}

// Additional utility functions for example management
export const exampleUtils = {
  /**
   * Get examples by category
   */
  async getByCategory(category) {
    const examplesPath = path.join(process.cwd(), 'public/examples.json');
    const examplesData = await fs.readFile(examplesPath, 'utf-8');
    const examples = JSON.parse(examplesData);

    return examples.examples?.filter((ex) => ex.category === category) || [];
  },

  /**
   * Search examples by term
   */
  async search(term) {
    const examplesPath = path.join(process.cwd(), 'public/examples.json');
    const examplesData = await fs.readFile(examplesPath, 'utf-8');
    const examples = JSON.parse(examplesData);

    const searchTerm = term.toLowerCase();
    return (
      examples.examples?.filter(
        (ex) =>
          ex.name.toLowerCase().includes(searchTerm) ||
          ex.description.toLowerCase().includes(searchTerm) ||
          ex.code.toLowerCase().includes(searchTerm),
      ) || []
    );
  },

  /**
   * Get random example
   */
  async getRandom() {
    const examplesPath = path.join(process.cwd(), 'public/examples.json');
    const examplesData = await fs.readFile(examplesPath, 'utf-8');
    const examples = JSON.parse(examplesData);

    const exampleList = examples.examples || [];
    const randomIndex = Math.floor(Math.random() * exampleList.length);
    return exampleList[randomIndex];
  },

  /**
   * Get examples statistics
   */
  async getStats() {
    const examplesPath = path.join(process.cwd(), 'public/examples.json');
    const examplesData = await fs.readFile(examplesPath, 'utf-8');
    const examples = JSON.parse(examplesData);

    const exampleList = examples.examples || [];
    const categories = [...new Set(exampleList.map((ex) => ex.category))];
    const authors = [...new Set(exampleList.map((ex) => ex.author).filter(Boolean))];

    return {
      total: exampleList.length,
      categories: categories.length,
      authors: authors.length,
      categoryBreakdown: categories.map((cat) => ({
        category: cat,
        count: exampleList.filter((ex) => ex.category === cat).length,
      })),
    };
  },
};
