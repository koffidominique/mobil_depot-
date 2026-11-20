/** PARTIE 13 — garde-fous contre les flottants et les données de démonstration. */
import { depot } from '../src/core/client';
import { formatFcfaExact, parseIntegerFcfa, parseIntegerQuantity, toJsonInteger } from '../src/core/format';

jest.mock('../src/specs/NativeDepotCore', () => ({
  __esModule: true,
  default: {
    executeCommand: jest.fn(() =>
      JSON.stringify({ ok: true, data: { database: '/tmp/base.sqlite', schema_version: 12, seeded_demo: false } }),
    ),
  },
}));

test('les quantités saisies sont entières et sans arrondi implicite', () => {
  expect(parseIntegerQuantity('12')).toBe(12n);
  expect(() => parseIntegerQuantity('12.5')).toThrow('QUANTITE_INVALIDE');
  expect(() => parseIntegerQuantity('-2')).toThrow('QUANTITE_INVALIDE');
  expect(toJsonInteger(parseIntegerQuantity('0007'))).toBe(7);
});

test('les montants restent des entiers FCFA', () => {
  expect(parseIntegerFcfa('12500')).toBe(12500n);
  expect(formatFcfaExact(12500n)).toBe('12 500 FCFA');
  expect(() => toJsonInteger(2n ** 60n)).toThrow('MONTANT_TROP_GRAND');
});

test('l’initialisation peut refuser les données de démonstration', async () => {
  const result = await depot.initializeDatabase('/tmp/base.sqlite', false);
  expect(result.seeded_demo).toBe(false);
});
