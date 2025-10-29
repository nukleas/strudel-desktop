/*
validation.ts - Register Strudel code validation handler for Tauri
Copyright (C) 2022 Strudel contributors
*/

import { validateStrudelCode } from '@strudel/desktopbridge/validation';

// Register validation handler for Tauri commands to call from Rust
if (typeof window !== 'undefined' && window.__TAURI_INTERNALS__) {
  // @ts-ignore - Adding custom property to window
  window.validateStrudelCodeHandler = validateStrudelCode;
  console.log('✅ Strudel validation handler registered for Tauri');
}

export { validateStrudelCode };
