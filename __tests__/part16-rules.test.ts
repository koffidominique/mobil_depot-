/** PARTIE 16 — règle 17 : aucune donnée de démonstration sur mobile natif. */
import { shouldSeedDemo } from '../src/core/environment';

test('le jeu pilote est réservé à l’aperçu web simulé', () => {
  expect(shouldSeedDemo('web')).toBe(true);
  expect(shouldSeedDemo('android')).toBe(false);
  expect(shouldSeedDemo('ios')).toBe(false);
});
