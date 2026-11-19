module.exports = {
  presets: ['module:@react-native/babel-preset'],
  // Zod 4 publie `export * as …` dans ses entrées ESM ; Metro/RN 0.88 a besoin de ce plugin.
  plugins: ['@babel/plugin-transform-export-namespace-from'],
};
