package com.example.core

import java.text.NumberFormat
import java.util.Locale

/**
 * Règle métier stricte (Lot 2) :
 * Tous les montants financiers sont exprimés en FCFA (XOF) entiers (Long),
 * sans aucune arithmétique flottante ni arrondi JavaScript/Double.
 */
object FcfaMath {
    fun checkedAdd(a: Long, b: Long, context: String = "addition FCFA"): Long {
        return try {
            Math.addExact(a, b)
        } catch (_: ArithmeticException) {
            throw DomainException("MONEY_OVERFLOW", "Dépassement de capacité lors de $context")
        }
    }

    fun checkedSub(a: Long, b: Long, context: String = "soustraction FCFA"): Long {
        return try {
            Math.subtractExact(a, b)
        } catch (_: ArithmeticException) {
            throw DomainException("MONEY_OVERFLOW", "Dépassement de capacité lors de $context")
        }
    }

    fun checkedMul(unitPriceFcfa: Long, quantity: Int, context: String = "multiplication FCFA"): Long {
        if (quantity < 0) {
            throw DomainException("NEGATIVE_QUANTITY", "Quantité négative interdite ($context)")
        }
        return try {
            Math.multiplyExact(unitPriceFcfa, quantity.toLong())
        } catch (_: ArithmeticException) {
            throw DomainException("MONEY_OVERFLOW", "Dépassement de capacité lors de $context")
        }
    }

    fun formatFcfa(amountFcfa: Long): String {
        val formatter = NumberFormat.getIntegerInstance(Locale.FRANCE)
        val formatted = formatter.format(amountFcfa).replace('\u00A0', ' ').replace('\u202F', ' ')
        return "$formatted FCFA"
    }

    fun formatCompactNumber(value: Long): String {
        val formatter = NumberFormat.getIntegerInstance(Locale.FRANCE)
        return formatter.format(value).replace('\u00A0', ' ').replace('\u202F', ' ')
    }
}

class DomainException(
    val code: String,
    override val message: String
) : IllegalArgumentException(message)

enum class CommercialUnit(val labelFr: String, val shortLabel: String) {
    CASE("Casier", "casier"),
    HALF_CASE("Demi-casier", "1/2 casier"),
    CARTON("Carton", "carton"),
    PACK("Pack", "pack"),
    BOTTLE("Bouteille", "btl"),
    CAN("Canette", "canette"),
    UNIT("Unité", "unité");

    companion object {
        fun fromCode(code: String): CommercialUnit {
            return entries.firstOrNull { it.name == code } ?: UNIT
        }
    }
}

enum class PriceTier(val labelFr: String) {
    RETAIL("Détail"),
    WHOLESALE("Gros");

    companion object {
        fun fromCode(code: String): PriceTier {
            return entries.firstOrNull { it.name == code } ?: RETAIL
        }
    }
}

enum class ContainerMaterial(val labelFr: String) {
    GLASS("Verre"),
    PET("PET"),
    ALUMINIUM("Aluminium"),
    OTHER("Autre");

    companion object {
        fun fromCode(code: String): ContainerMaterial {
            return entries.firstOrNull { it.name == code } ?: OTHER
        }
    }
}

enum class UserRole(
    val labelFr: String,
    val maxDiscountPercent: Int,
    val canApproveStockAdjustment: Boolean,
    val canManagePurchases: Boolean,
    val canManageCashSession: Boolean,
    val canResolveConflicts: Boolean,
    val canRevokeDevices: Boolean,
    val canSell: Boolean
) {
    OWNER("Propriétaire", 100, true, true, true, true, true, true),
    ADMIN("Administrateur", 50, true, true, true, true, true, true),
    MANAGER("Gérant", 25, true, true, true, true, false, true),
    CASHIER("Caissier", 5, false, false, true, false, false, true),
    WAREHOUSE("Magasinier", 0, true, true, false, false, false, false),
    ACCOUNTANT("Comptable", 0, false, false, true, false, false, false),
    READ_ONLY("Consultation", 0, false, false, false, false, false, false);

    companion object {
        fun fromCode(code: String): UserRole {
            return entries.firstOrNull { it.name == code } ?: READ_ONLY
        }
    }
}

enum class OfflineSalePolicy(val labelFr: String) {
    ALLOW_WITH_WARNING("Autoriser avec avertissement de stock non garanti"),
    STRICT_LOCAL_BLOCK("Bloquer si stock local insuffisant");

    companion object {
        fun fromCode(code: String): OfflineSalePolicy {
            return entries.firstOrNull { it.name == code } ?: ALLOW_WITH_WARNING
        }
    }
}

/**
 * Calcule le nombre entier physique de bouteilles/canettes/unités.
 * Interdit strictement toute fraction physique de bouteille.
 */
object UnitQuantityCalculator {
    fun toPhysicalUnits(
        unit: CommercialUnit,
        unitCount: Int,
        crateCapacity: Int,
        packSize: Int,
        allowHalfCase: Boolean
    ): Int {
        if (unitCount <= 0) {
            throw DomainException("INVALID_QUANTITY", "La quantité doit être un entier strictement positif.")
        }
        return when (unit) {
            CommercialUnit.CASE -> {
                if (crateCapacity <= 0) {
                    throw DomainException("INVALID_CRATE_CAPACITY", "Capacité du casier non configurée.")
                }
                Math.multiplyExact(unitCount, crateCapacity)
            }
            CommercialUnit.HALF_CASE -> {
                if (!allowHalfCase) {
                    throw DomainException("HALF_CASE_FORBIDDEN", "Le demi-casier n'est pas autorisé pour ce produit.")
                }
                if (crateCapacity <= 0 || crateCapacity % 2 != 0) {
                    throw DomainException(
                        "NON_INTEGER_HALF_CASE",
                        "Impossible de diviser un casier de $crateCapacity emplacements en demi-casier entier."
                    )
                }
                val half = crateCapacity / 2
                Math.multiplyExact(unitCount, half)
            }
            CommercialUnit.PACK, CommercialUnit.CARTON -> {
                val size = if (packSize > 0) packSize else 1
                Math.multiplyExact(unitCount, size)
            }
            CommercialUnit.BOTTLE, CommercialUnit.CAN, CommercialUnit.UNIT -> unitCount
        }
    }
}
