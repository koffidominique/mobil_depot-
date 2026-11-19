import React from 'react';
import { createRoot } from 'react-dom/client';
import App from '../src/App';

const root = document.getElementById('root');
if (!root) throw new Error('Élément #root introuvable');
createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
