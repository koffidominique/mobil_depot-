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

export const mixedCaseLineSchema = z.object({
  variant_id: z.string().min(1),
  variant_name: z.string().min(1),
  quantity_bottles: z.number().int().positive('Quantité entière positive requise.'),
  unit_price_fcfa: z.number().int().nonnegative(),
  compatibility_group_id: z.string().min(1, 'Groupe physique requis pour un casier mixte.'),
  bottle_profile_id: z.string().min(1, 'Profil de bouteille requis pour un casier mixte.'),
  is_reusable: z.boolean(),
});

export const mixedCaseSchema = z.object({
  crate_type_id: z.string().min(1, 'Format de casier requis.'),
  is_half_case: z.boolean(),
  lines: z.array(mixedCaseLineSchema).min(1, 'Ajoutez au moins une boisson.'),
});

export const saleSchema = z.object({
  customer_id: z.string().nullable(),
  price_tier: z.enum(['RETAIL', 'WHOLESALE']),
  lines: z.array(cartLineSchema),
  mixed_cases: z.array(mixedCaseSchema),
  // Fiche de clientèle : un client identifié par son nom (et son téléphone) est
  // créé ou retrouvé automatiquement ; aucune vente n'est anonyme.
  customer_name: z.string().trim().min(2, 'Nom du client requis.').optional(),
  customer_phone: z.string().trim().min(1).optional(),
  customer_neighborhood: z.string().trim().min(1).optional(),
  // Clé d'idempotence : une vente rejouée (retry réseau, double appui) ne doit
  // jamais produire un second ticket ni un second mouvement de casiers.
  request_key: z.string().min(8).optional(),
  discount_fcfa: z.number().int().nonnegative(),
  payments: z.array(paymentSchema),
  is_online: z.boolean(),
  // Casiers vides rendus immédiatement par le client : retour physique, jamais financier.
  empty_crates_returned: z
    .array(z.object({
      crate_type_id: z.string().min(1),
      qty: z.number().int().positive(),
      reason: z.string().optional(),
    }))
    .optional(),
});

export type LoginForm = z.infer<typeof loginSchema>;
