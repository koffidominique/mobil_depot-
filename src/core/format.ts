const integerFormatter = new Intl.NumberFormat('fr-CI', { maximumFractionDigits: 0 });

/** Montants FCFA : aucun appel n'accepte de nombre décimal. */
export function formatFcfa(amountFcfa: number): string {
  if (!Number.isSafeInteger(amountFcfa)) {
    throw new Error('MONTANT_FCFA_INVALIDE');
  }
  return `${integerFormatter.format(amountFcfa)} FCFA`;
}

export function formatQuantity(value: number): string {
  if (!Number.isSafeInteger(value)) {
    throw new Error('QUANTITE_INVALIDE');
  }
  return integerFormatter.format(value);
}

export function formatTimestamp(epochMs: number): string {
  return new Intl.DateTimeFormat('fr-CI', {
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(epochMs));
}

/** Formatage exact sans passage par `Number` : FCFA entiers uniquement. */
export function formatFcfaExact(amountFcfa: bigint | number): string {
  const value = BigInt(amountFcfa);
  const digits = value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
  return `${digits} FCFA`;
}

/** Analyse stricte d'un montant saisi : aucun flottant, aucun arrondi. */
export function parseIntegerFcfa(input: string): bigint {
  if (!/^\d+$/.test(input.trim())) {
    throw new Error('MONTANT_FCFA_INVALIDE');
  }
  return BigInt(input.trim());
}

/** borne de sérialisation : le calcul reste en BigInt, seul le DTO JSON est un entier JS. */
export function toJsonInteger(value: bigint): number {
  if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error('MONTANT_TROP_GRAND');
  }
  return Number(value);
}

/** Quantité entière saisie dans un champ texte : aucun passage par un flottant. */
export function parseIntegerQuantity(input: string): bigint {
  if (!/^\d+$/.test(input.trim())) {
    throw new Error('QUANTITE_INVALIDE');
  }
  return BigInt(input.trim());
}
