/** Vérification headless de l'aperçu web : bundle + rendu React dans JSDOM. */
import { build } from 'esbuild';
import { JSDOM, VirtualConsole } from 'jsdom';
import path from 'node:path';
import fs from 'node:fs';

const root = path.resolve(import.meta.dirname, '..');
const shim = (name) => path.join(root, 'web/shims', name);
const alias = {
  'react-native': shim('react-native.ts'),
  'react-native-gesture-handler': shim('empty.ts'),
  'react-native-safe-area-context': shim('safe-area.tsx'),
  '@react-native-community/netinfo': shim('netinfo.ts'),
  'react-native-fs': shim('fs.ts'),
  'react-native-keychain': shim('keychain.ts'),
  '@react-navigation/native': shim('navigation.tsx'),
  '@react-navigation/bottom-tabs': shim('navigation.tsx'),
  '@react-navigation/native-stack': shim('navigation.tsx'),
};
const outfile = path.join(root, '.web-preview-check.js');
await build({
  absWorkingDir: root,
  entryPoints: ['web/main.tsx'],
  bundle: true,
  outfile,
  format: 'iife',
  platform: 'browser',
  target: 'es2022',
  alias,
  define: { 'process.env.NODE_ENV': '"development"' },
  logLevel: 'silent',
  plugins: [{
    name: 'native-mocks',
    setup(build) {
      build.onResolve({ filter: /NativeDepotCore$/ }, () => ({ path: path.join(root, 'web/mock/NativeDepotCore.ts') }));
      build.onResolve({ filter: /NativeReceiptPrinter$/ }, () => ({ path: path.join(root, 'web/mock/NativeReceiptPrinter.ts') }));
    },
  }],
});

const code = fs.readFileSync(outfile, 'utf8');
fs.rmSync(outfile, { force: true });
const virtualConsole = new VirtualConsole();
const errors = [];
virtualConsole.on('jsdomError', (error) => errors.push(error.message));
virtualConsole.on('error', (message) => errors.push(String(message)));
const dom = new JSDOM('<!doctype html><html><body><div id="root"></div></body></html>', {
  url: 'https://preview.local/',
  runScripts: 'outside-only',
  pretendToBeVisual: true,
  virtualConsole,
});
const { window } = dom;
window.addEventListener('error', (event) => errors.push(event.message || String(event.error)));
window.matchMedia = () => ({ matches: false, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {} });
window.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
window.scrollTo = () => {};
try {
  window.eval(code);
  await new Promise((resolve) => setTimeout(resolve, 2500));
  const text = window.document.body.textContent ?? '';
  const expected = ['DEPOTPRO', 'Se connecter', 'Code PIN'];
  const missing = expected.filter((token) => !text.includes(token));
  console.log(`Connexion : ${text.length} caractères · ${errors.length} erreur(s)`);
  if (!missing.length) {
    const inputs = [...window.document.querySelectorAll('input')];
    const pin = inputs.find((input) => input.type === 'password');
    if (pin) {
      Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set.call(pin, '1234');
      pin.dispatchEvent(new window.Event('input', { bubbles: true }));
    }
    const button = [...window.document.querySelectorAll('[role="button"],button')]
      .find((element) => (element.textContent ?? '').includes('Se connecter'));
    button?.dispatchEvent(new window.MouseEvent('click', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 2500));
  }
  // Parcours des écrans du lot 49 : Stock, Inventaires, Produits et Commandes
  // clients doivent s'ouvrir sans erreur, avec leurs données, dans l'aperçu.
  const clickBy = async (selector, fragment) => {
    const node = [...window.document.querySelectorAll(selector)]
      .find((element) => (element.textContent ?? '').includes(fragment));
    if (!node) return false;
    node.dispatchEvent(new window.MouseEvent('click', { bubbles: true }));
    await new Promise((resolve) => setTimeout(resolve, 900));
    return true;
  };
  const walk = [];
  const visit = async (label, selector, fragment, expected, back = false) => {
    const clicked = await clickBy(selector, fragment);
    const text = window.document.body.textContent ?? '';
    walk.push({ label, ok: clicked && text.includes(expected), expected, found: text.includes(expected) });
    if (back) await clickBy('[role="button"]', '←');
  };
  await visit('Stock', '[role="tab"]', 'Stock', "Vue d'ensemble");
  await visit('Inventaires', '[role="button"]', 'Inventaires physiques : compter', 'Nouvelle session de comptage', true);
  await visit('Menu', '[role="tab"]', 'Menu', 'Verrouiller la session');
  await visit('Produits', '[role="button"]', 'Produits & catalogue', 'Références', true);
  await visit('Stock (retour)', '[role="tab"]', 'Stock', "Vue d'ensemble");
  await visit('Commandes clients', '[role="button"]', 'Commandes clients : reçus', 'Sélection', true);
  for (const entry of walk) {
    console.log(
      `Écran ${entry.label} : ${entry.ok ? 'OK' : 'ÉCHEC'} (attendu « ${entry.expected} » ${entry.found ? 'présent' : 'ABSENT'})`,
    );
  }
  if (walk.some((entry) => !entry.ok)) process.exitCode = 1;

  const signedInText = window.document.body.textContent ?? '';
  const missingSignedIn = ['Accueil', 'Ventes', 'Menu'].filter((token) => !signedInText.includes(token));
  console.log(`Après connexion : ${signedInText.length} caractères · ${errors.length} erreur(s) · ${JSON.stringify(signedInText.slice(0, 140))}`);
  if (errors.length || missing.length || missingSignedIn.length) {
    console.error('Erreurs :', errors.slice(0, 5));
    console.error('Éléments absents :', missing, missingSignedIn);
    process.exitCode = 1;
  } else {
    console.log('OK : connexion, navigation et modules natifs simulés vérifiés.');
  }
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  window.close();
}
