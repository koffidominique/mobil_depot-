import { formatFcfa } from '../src/core/format';
import { cartLineSchema, loginSchema } from '../src/core/schemas';

describe('contrats interface fr-CI', () => {
  it('formate les montants FCFA sans décimale', () => {
    expect(formatFcfa(1234567)).toMatch(/1\s?234\s?567 FCFA/u);
    expect(() => formatFcfa(12.5)).toThrow('MONTANT_FCFA_INVALIDE');
  });

  it('exige un PIN à quatre chiffres', () => {
    expect(loginSchema.safeParse({ username: 'gerant', pin: '1234', branchId: 'b1' }).success).toBe(true);
    expect(loginSchema.safeParse({ username: 'gerant', pin: '123', branchId: 'b1' }).success).toBe(false);
  });

  it('refuse une quantité fractionnaire de bouteilles', () => {
    const line = {
      line_key: 'v1_BOTTLE',
      variant_id: 'v1',
      variant_name: 'Beaufort',
      unit: 'BOTTLE',
      unit_count: 1.5,
      unit_price_fcfa: 600,
      crate_capacity: 12,
      pack_size: 12,
      is_reusable: true,
      allow_half_case: true,
      bottle_profile_id: 'bp_1',
      crate_type_id: 'crate_b12_50',
    };
    expect(cartLineSchema.safeParse(line).success).toBe(false);
  });
});
