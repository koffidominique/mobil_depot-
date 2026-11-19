import { z } from 'zod';

export const loginSchema = z.object({
  username: z.string().min(2, 'Identifiant requis.'),
  pin: z.string().regex(/^\d{4}$/, 'Le code PIN comporte 4 chiffres.'),
  branchId: z.string().min(1),
});

export const cartLineSchema = z.object({
  line_key: z.string().min(1),
  variant_id: z.string().min(1),
  variant_name: z.string().min(1),
  unit: z.enum(['CASE', 'HALF_CASE', 'CARTON', 'PACK', 'BOTTLE', 'CAN', 'UNIT']),
  unit_count: z.number().int().positive('Quantité entière positive requise.'),
  unit_price_fcfa: z.number().int().nonnegative(),
  crate_capacity: z.number().int().positive(),
  pack_size: z.number().int().positive(),
  is_reusable: z.boolean(),
  allow_half_case: z.boolean(),
  bottle_profile_id: z.string().nullable(),
  crate_type_id: z.string().nullable(),
});

export const paymentSchema = z.object({
  method: z.enum(['CASH', 'MOBILE_MONEY', 'BANK_TRANSFER', 'CHECK', 'CREDIT']),
  amount_fcfa: z.number().int().nonnegative(),
  reference: z.string().nullable(),
  confirmed_by_provider: z.boolean(),
});

export const saleSchema = z.object({
  customer_id: z.string().nullable(),
  price_tier: z.enum(['RETAIL', 'WHOLESALE']),
  lines: z.array(cartLineSchema),
  mixed_cases: z.array(z.unknown()),
  discount_fcfa: z.number().int().nonnegative(),
  payments: z.array(paymentSchema),
  is_online: z.boolean(),
});

export type LoginForm = z.infer<typeof loginSchema>;
