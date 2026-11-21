/**
 * Clés d'idempotence côté application.
 *
 * Toute opération de casiers qui modifie plusieurs tables (journal, stock,
 * soldes physiques, caisse) exige un `request_key`. La clé est générée une
 * seule fois puis rejouée telle quelle en cas de nouvelle tentative : le
 * backend reconnaît l'opération et ne la comptabilise pas deux fois.
 */
let counter = 0;

export function newRequestKey(scope: string): string {
  counter = (counter + 1) % 100000;
  const stamp = Date.now().toString(36);
  const rand = Math.random().toString(36).slice(2, 8);
  const device = Math.random().toString(36).slice(2, 6);
  return `${scope}:${stamp}:${counter}:${device}:${rand}`;
}
