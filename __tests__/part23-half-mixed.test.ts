/**
 * Demi-casier et casier mixte dans la vente (lot 45).
 *
 * Vérifie : la règle de prix du demi-casier (prix bouteille × capacité ÷ 2), la
 * lecture de la capacité et de l'autorisation de demi-casier **dans le
 * référentiel** (le client ne les fixe pas), la persistance effective des
 * casiers mixtes dans la vente, et le fait que `createMixedOrder` ne crée rien.
 */
import { canSellHalfCase, halfCasePrice, halfCaseUnits } from '../src/core/pricing';
import { executePreviewCommand } from '../web/mock/engine';

interface Envelope {
  ok: boolean;
  data?: any;
  error?: { code: string; message: string };
}

function call(command: string, payload: Record<string, unknown> = {}): any {
  const parsed = JSON.parse(executePreviewCommand(JSON.stringify({ command, payload }))) as Envelope;
  if (!parsed.ok) throw new Error(`${parsed.error?.code}: ${parsed.error?.message}`);
  return parsed.data;
}

function attempt(command: string, payload: Record<string, unknown> = {}): Envelope {
  return JSON.parse(executePreviewCommand(JSON.stringify({ command, payload }))) as Envelope;
}

function saleLine(overrides: Record<string, unknown> = {}) {
  return {
    line_key: 'L1', variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl',
    unit: 'BOTTLE', unit_count: 1, unit_price_fcfa: 600,
    crate_capacity: 12, pack_size: 12, is_reusable: true, allow_half_case: true,
    bottle_profile_id: 'bp_drogba_50', crate_type_id: 'crate_b12_50',
    ...overrides,
  };
}

function salePayload(extra: Record<string, unknown> = {}) {
  return {
    customer_id: 'cust_palmier', price_tier: 'RETAIL', lines: [], mixed_cases: [],
    discount_fcfa: 0, payments: [], is_online: true, ...extra,
  };
}

const HALF_LINE = saleLine({ line_key: 'HALF1', unit: 'HALF_CASE', unit_count: 1, unit_price_fcfa: 3600 });

describe('règle de prix du demi-casier', () => {
  it('vaut le prix bouteille × (unités du casier ÷ 2)', () => {
    expect(halfCaseUnits({ pack_size: 12 })).toBe(6);
    expect(halfCaseUnits({ pack_size: 24 })).toBe(12);
    expect(halfCasePrice({ pack_size: 12, retail_bottle_fcfa: 600 })).toBe(3600);
    expect(halfCasePrice({ pack_size: 24, retail_bottle_fcfa: 350 })).toBe(4200);
  });

  it('refuse un demi-casier sans casier physique, impair ou interdit', () => {
    expect(canSellHalfCase({ pack_size: 12, crate_type_id: 'crate_b12_50' })).toBe(true);
    expect(canSellHalfCase({ pack_size: 12, crate_type_id: null })).toBe(false);
    expect(canSellHalfCase({ pack_size: 5, crate_type_id: 'crate_b12_50' })).toBe(false);
    expect(canSellHalfCase({ pack_size: 12, crate_type_id: 'crate_b12_50', allow_half_case: false })).toBe(false);
  });
});

describe('vente au demi-casier', () => {
  it('sort 6 bouteilles, garde le casier au dépôt et ne crée aucune dette', () => {
    const result = call('createSale', salePayload({ lines: [HALF_LINE] }));
    expect(result.total_fcfa).toBe(3600);
    const movement = (result.crate_movements as any[])[0];
    expect(movement.movement_type).toBe('sale_full_out');
    expect(movement.qty).toBe(1);
    // Le casier plein devient un casier vide au dépôt : rien ne part chez le client.
    expect(movement.full_after).toBe(movement.full_before - 1);
    expect(movement.empty_after).toBe(movement.empty_before + 1);
    expect(movement.partner_delta).toBe(0);
    expect(movement.partner_type ?? null).toBeNull();
    expect(movement.destination).toBe('DEPOT');
    expect(movement.reason).toContain('Demi-casier');
  });

  it('la quantité physique est calculée sur la capacité réelle du casier', () => {
    const validated = call('validateSale', salePayload({ lines: [HALF_LINE] }));
    expect(validated.physical_units_by_variant.var_drogba_50).toBe(6);
    expect(validated.total_fcfa).toBe(3600);
  });

  it('refuse le demi-casier quand le format l’interdit, même si le client prétend le contraire', () => {
    call('updateCrateType', { crate_type_id: 'crate_b6_150', allow_half_case: false });
    try {
      const refused = attempt('createSale', salePayload({
        lines: [saleLine({
          line_key: 'HALF_CELESTE', variant_id: 'var_celeste_150', variant_name: 'Céleste 1,5 L',
          unit: 'HALF_CASE', unit_count: 1, unit_price_fcfa: 4500,
          crate_capacity: 6, pack_size: 6, crate_type_id: 'crate_b6_150',
          bottle_profile_id: 'bp_celeste_150', allow_half_case: true,
        })],
      }));
      expect(refused.ok).toBe(false);
      expect(refused.error?.code).toBe('HALF_CASE_NOT_ALLOWED');
    } finally {
      call('updateCrateType', { crate_type_id: 'crate_b6_150', allow_half_case: true });
    }
  });

  it('refuse un demi-casier sur un produit sans casier physique', () => {
    const refused = attempt('createSale', salePayload({
      lines: [saleLine({
        line_key: 'HALF_CAN', variant_id: 'var_flag_can_33', variant_name: 'Flag Canette 33 cl',
        unit: 'HALF_CASE', unit_price_fcfa: 1080, crate_capacity: 24, pack_size: 24,
        crate_type_id: null, bottle_profile_id: null,
      })],
    }));
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('HALF_CASE_NOT_ALLOWED');
  });
});

const MIXED_CASE = {
  crate_type_id: 'crate_b12_50',
  is_half_case: false,
  lines: [
    {
      variant_id: 'var_drogba_50', variant_name: 'Drogba 50 cl', quantity_bottles: 6,
      unit_price_fcfa: 600, compatibility_group_id: 'G_B12_50',
      bottle_profile_id: 'bp_drogba_50', is_reusable: true,
    },
    {
      variant_id: 'var_beaufort_50', variant_name: 'Beaufort 50 cl', quantity_bottles: 6,
      unit_price_fcfa: 650, compatibility_group_id: 'G_B12_50',
      bottle_profile_id: 'bp_beaufort_50', is_reusable: true,
    },
  ],
};

describe('casier mixte dans la vente', () => {
  it('valide un casier complet sans rien écrire', () => {
    const salesBefore = call('getSales').sales.length;
    const movementsBefore = call('getCrateMovementHistory', {}).movements.length;

    const validated = call('createMixedOrder', MIXED_CASE);
    expect(validated.crate_code).toBe('B12-50');
    expect(validated.slot_capacity).toBe(12);
    expect(validated.total_drinks_fcfa).toBe(7500);
    expect(validated.validated_only).toBe(true);

    expect(call('getSales').sales.length).toBe(salesBefore);
    expect(call('getCrateMovementHistory', {}).movements.length).toBe(movementsBefore);
  });

  it('vend réellement le casier mixte et sort un casier chez le client', () => {
    const result = call('createSale', salePayload({ mixed_cases: [MIXED_CASE] }));
    expect(result.total_fcfa).toBe(7500);
    const movement = (result.crate_movements as any[])[0];
    expect(movement.crate_type_id).toBe('crate_b12_50');
    expect(movement.full_after).toBe(movement.full_before - 1);
    expect(movement.partner_delta).toBe(1);
    expect(movement.partner_type).toBe('CUSTOMER');
    expect(movement.destination).toBe('CLIENT');
    expect(movement.reason).toContain('Casier mixte');

    const sale = call('getSales').sales[0];
    const detail = call('getSaleReceipt', { sale_id: sale.id });
    expect(detail.lines.filter((line: any) => line.is_mixed).length).toBe(2);
  });

  it('un casier mixte incomplet est refusé', () => {
    const refused = attempt('createSale', salePayload({
      mixed_cases: [{ ...MIXED_CASE, lines: [{ ...MIXED_CASE.lines[0]!, quantity_bottles: 5 }, { ...MIXED_CASE.lines[1]!, quantity_bottles: 5 }] }],
    }));
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('MIXED_CASE_CAPACITY_MISMATCH');
  });

  it('un profil physique incompatible est refusé, même à volume égal', () => {
    const incompatible = attempt('createSale', salePayload({
      mixed_cases: [{
        crate_type_id: 'crate_b12_50',
        is_half_case: false,
        lines: [
          { ...MIXED_CASE.lines[0]!, quantity_bottles: 6 },
          {
            variant_id: 'var_flag_50', variant_name: 'Flag 50 cl', quantity_bottles: 6,
            unit_price_fcfa: 620, compatibility_group_id: 'G_B12_50_ALT',
            bottle_profile_id: 'bp_flag_50', is_reusable: true,
          },
        ],
      }],
    }));
    expect(incompatible.ok).toBe(false);
    expect(incompatible.error?.code).toBe('INCOMPATIBLE_CRATE_PROFILE');
  });

  it('un casier mixte sur un format inconnu est refusé', () => {
    const refused = attempt('createSale', salePayload({
      mixed_cases: [{ ...MIXED_CASE, crate_type_id: 'crate_inexistant' }],
    }));
    expect(refused.ok).toBe(false);
    expect(refused.error?.code).toBe('CRATE_NOT_FOUND');
  });

  it('un demi-casier mixte reste au dépôt (casier ouvert, aucun solde client)', () => {
    const halfCasePayload = {
      crate_type_id: 'crate_b12_50',
      is_half_case: true,
      lines: [{ ...MIXED_CASE.lines[0]!, quantity_bottles: 6 }],
    };
    const validated = call('createMixedOrder', halfCasePayload);
    expect(validated.slot_capacity).toBe(6);

    const result = call('createSale', salePayload({ mixed_cases: [halfCasePayload] }));
    const movement = (result.crate_movements as any[])[0];
    expect(movement.full_after).toBe(movement.full_before - 1);
    expect(movement.empty_after).toBe(movement.empty_before + 1);
    expect(movement.partner_delta).toBe(0);
    expect(movement.destination).toBe('DEPOT');
    expect(result.total_fcfa).toBe(3600);
  });

  it('un emballage jetable ne peut pas entrer dans un casier mixte', () => {
    const disposable = attempt('createSale', salePayload({
      mixed_cases: [{
        crate_type_id: 'crate_b12_50',
        is_half_case: false,
        lines: [
          { ...MIXED_CASE.lines[0]!, is_reusable: false },
          { ...MIXED_CASE.lines[1]! },
        ],
      }],
    }));
    expect(disposable.ok).toBe(false);
    expect(disposable.error?.code).toBe('DISPOSABLE_IN_MIXED_CRATE');
  });
});
