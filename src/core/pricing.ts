import type { InventoryItem } from './types';

/**
 * Règles de prix du **demi-casier** (décision OWNER, lot 45).
 *
 * Un demi-casier n'a pas de prix propre en base : il vaut
 * `prix bouteille × (unités du casier ÷ 2)`. La capacité utilisée est le nombre
 * d'unités réellement porté par la variante (`pack_size`), jamais une valeur
 * supposée : un casier impair ou sans casier physique interdit le demi-casier.
 */

export function halfCaseUnits(item: Pick<InventoryItem, 'pack_size'>): number {
  return Math.floor(item.pack_size / 2);
}

export function canSellHalfCase(
  item: Pick<InventoryItem, 'pack_size' | 'crate_type_id'> & { allow_half_case?: boolean },
): boolean {
  if (!item.crate_type_id) return false;
  if (item.allow_half_case === false) return false;
  return item.pack_size >= 2 && item.pack_size % 2 === 0;
}

export function halfCasePrice(
  item: Pick<InventoryItem, 'pack_size' | 'retail_bottle_fcfa'>,
): number {
  return item.retail_bottle_fcfa * halfCaseUnits(item);
}
