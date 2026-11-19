import { defineConfig } from 'vite';
import path from 'node:path';

const shim = (name: string) => path.resolve(__dirname, 'web/shims', name);

export default defineConfig({
  root: path.resolve(__dirname, 'web'),
  server: { host: '0.0.0.0', port: 5173, strictPort: true, allowedHosts: true, fs: { allow: [__dirname] } },
  preview: { host: '0.0.0.0', port: 5173, strictPort: true, allowedHosts: true },
  resolve: {
    extensions: ['.web.tsx', '.web.ts', '.tsx', '.ts', '.jsx', '.js', '.json'],
    alias: [
      { find: /^react-native$/, replacement: shim('react-native.ts') },
      { find: /^react-native-gesture-handler$/, replacement: shim('empty.ts') },
      { find: /^react-native-safe-area-context$/, replacement: shim('safe-area.tsx') },
      { find: /^@react-native-community\/netinfo$/, replacement: shim('netinfo.ts') },
      { find: /^react-native-fs$/, replacement: shim('fs.ts') },
      { find: /^react-native-keychain$/, replacement: shim('keychain.ts') },
      { find: /^@react-navigation\/native$/, replacement: shim('navigation.tsx') },
      { find: /^@react-navigation\/bottom-tabs$/, replacement: shim('navigation.tsx') },
      { find: /^@react-navigation\/native-stack$/, replacement: shim('navigation.tsx') },
      { find: /^.*\/NativeDepotCore$/, replacement: path.resolve(__dirname, 'web/mock/NativeDepotCore.ts') },
      { find: /^.*\/NativeReceiptPrinter$/, replacement: path.resolve(__dirname, 'web/mock/NativeReceiptPrinter.ts') },
    ],
  },
  optimizeDeps: { include: ['react-native-web'] },
});
