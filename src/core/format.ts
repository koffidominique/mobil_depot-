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
