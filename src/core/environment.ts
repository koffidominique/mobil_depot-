/** Règle 17 : seuls les aperçus web simulés peuvent amorcer un jeu de démonstration. */
export function shouldSeedDemo(platform: string): boolean {
  return platform === 'web';
}
