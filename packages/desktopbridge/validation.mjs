/*
validation.mjs - Strudel code validation for desktop app
Copyright (C) 2022 Strudel contributors - see <https://codeberg.org/uzu/strudel/src/branch/main/packages/desktopbridge/validation.mjs>
This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Affero General Public License for more details. You should have received a copy of the GNU Affero General Public License along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

import { transpiler } from '@strudel/transpiler';
import { evaluate } from '@strudel/core';

/**
 * Validate Strudel code by transpiling and evaluating it
 * @param {string} code - The Strudel code to validate
 * @returns {Promise<Object>} Validation result with valid (boolean), error (string|undefined), and stack (string|undefined)
 */
export async function validateStrudelCode(code) {
  try {
    if (!code || typeof code !== 'string') {
      return {
        valid: false,
        error: 'Code must be a non-empty string',
      };
    }

    // Step 1: Transpile (catches syntax errors)
    let transpiled;
    try {
      transpiled = transpiler(code, {
        wrapAsync: false,
        addReturn: true,
        emitMiniLocations: false, // Skip location tracking for validation
        emitWidgets: false, // Skip widgets for validation
      });
    } catch (err) {
      return {
        valid: false,
        error: `Syntax error: ${err.message}`,
        stack: err.stack,
      };
    }

    // Step 2: Evaluate (catches runtime errors)
    let result;
    try {
      result = await evaluate(transpiled.output, null, {});
    } catch (err) {
      return {
        valid: false,
        error: `Runtime error: ${err.message}`,
        stack: err.stack,
      };
    }

    // Step 3: Check if result is a Pattern
    const { pattern } = result;
    if (!pattern || !pattern._Pattern) {
      return {
        valid: false,
        error: 'Code did not return a Pattern object. Did you forget to call a function?',
      };
    }

    return { valid: true };
  } catch (err) {
    // Catch-all for unexpected errors
    return {
      valid: false,
      error: `Validation error: ${err.message}`,
      stack: err.stack,
    };
  }
}
