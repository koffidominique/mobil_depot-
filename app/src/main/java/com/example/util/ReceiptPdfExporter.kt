package com.example.util

import android.content.Context
import android.content.Intent
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.pdf.PdfDocument
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import androidx.core.content.FileProvider
import com.example.core.FcfaMath
import com.example.data.local.entities.SaleEntity
import com.example.data.local.entities.SaleLineEntity
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

data class ReceiptPackagingLine(
    val label: String,
    val expectedQty: Int,
    val returnedQty: Int
) {
    val missingQty: Int get() = (expectedQty - returnedQty).coerceAtLeast(0)
}

object ReceiptPdfExporter {

    private fun generateReceiptPdfFile(
        context: Context,
        depotTitle: String,
        branchName: String,
        depotPhone: String,
        sale: SaleEntity,
        lines: List<SaleLineEntity>,
        packagingLines: List<ReceiptPackagingLine>
    ): File {
        val dir = File(context.cacheDir, "receipts").apply { mkdirs() }
        val file = File(dir, "Recu_${sale.receiptNumber.replace('/', '_')}.pdf")

        val doc = PdfDocument()
        val pageInfo = PdfDocument.PageInfo.Builder(595, 842, 1).create()
        val page = doc.startPage(pageInfo)
        val canvas = page.canvas

        val titlePaint = Paint().apply {
            textSize = 18f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
            color = android.graphics.Color.parseColor("#0F1E2E")
        }
        val sectionPaint = Paint().apply {
            textSize = 13f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
            color = android.graphics.Color.parseColor("#D98218")
        }
        val bodyPaint = Paint().apply {
            textSize = 11f
            color = android.graphics.Color.parseColor("#1E293B")
        }
        val boldBodyPaint = Paint().apply {
            textSize = 11f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
            color = android.graphics.Color.parseColor("#0F1E2E")
        }
        val mutedPaint = Paint().apply {
            textSize = 10f
            color = android.graphics.Color.parseColor("#64748B")
        }

        val df = SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.FRANCE)
        var y = 48f
        val left = 40f

        canvas.drawText("DEPOTPRO — $depotTitle", left, y, titlePaint)
        y += 20f
        canvas.drawText("$branchName • Tél: $depotPhone", left, y, mutedPaint)
        y += 26f
        canvas.drawText("REÇU COMMERCIAL N° ${sale.receiptNumber}", left, y, boldBodyPaint)
        y += 16f
        canvas.drawText("Date : ${df.format(Date(sale.occurredAtMs))}  •  Caissier : ${sale.cashierName}", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Client : ${sale.customerName}  •  Tarif : ${sale.priceTier}", left, y, bodyPaint)
        y += 24f

        canvas.drawText("BOISSONS & ARTICLES", left, y, sectionPaint)
        y += 18f
        for (line in lines) {
            val text = "${line.unitCount}x ${line.unitCode} • ${line.variantName}"
            val amt = FcfaMath.formatFcfa(line.lineTotalFcfa)
            canvas.drawText(text.take(46), left, y, bodyPaint)
            canvas.drawText(amt, 430f, y, boldBodyPaint)
            y += 16f
        }

        y += 12f
        canvas.drawText("Sous-total : ${FcfaMath.formatFcfa(sale.subtotalFcfa)}", left, y, bodyPaint)
        y += 15f
        if (sale.discountFcfa > 0L) {
            canvas.drawText("Remise : -${FcfaMath.formatFcfa(sale.discountFcfa)}", left, y, bodyPaint)
            y += 15f
        }
        canvas.drawText("NET À PAYER : ${FcfaMath.formatFcfa(sale.totalFcfa)}", left, y, titlePaint)
        y += 18f
        canvas.drawText("Encaissé confirmé : ${FcfaMath.formatFcfa(sale.paidConfirmedFcfa)}", left, y, bodyPaint)
        if (sale.pendingMobileMoneyFcfa > 0L) {
            y += 15f
            canvas.drawText("Mobile Money en attente : ${FcfaMath.formatFcfa(sale.pendingMobileMoneyFcfa)}", left, y, bodyPaint)
        }
        if (sale.creditFcfa > 0L) {
            y += 15f
            canvas.drawText("Reste à crédit : ${FcfaMath.formatFcfa(sale.creditFcfa)}", left, y, boldBodyPaint)
        }

        y += 26f
        canvas.drawText("SECTION EMBALLAGES CONSIGNÉS (HORS VALEUR BOISSONS)", left, y, sectionPaint)
        y += 18f
        if (packagingLines.isEmpty()) {
            canvas.drawText("Aucun emballage réutilisable attendu sur cette vente.", left, y, mutedPaint)
            y += 16f
        } else {
            for (pkg in packagingLines) {
                canvas.drawText(
                    "${pkg.label} — Attendu: ${pkg.expectedQty} | Reçu: ${pkg.returnedQty} | Restant dû: ${pkg.missingQty}",
                    left,
                    y,
                    bodyPaint
                )
                y += 16f
            }
        }

        y += 24f
        canvas.drawText(
            "Document commercial interne (ne tient pas lieu de facture normalisée fiscale).",
            left,
            y,
            mutedPaint
        )

        doc.finishPage(page)
        FileOutputStream(file).use { doc.writeTo(it) }
        doc.close()
        return file
    }

    fun exportAndShareReceipt(
        context: Context,
        depotTitle: String,
        branchName: String,
        depotPhone: String,
        sale: SaleEntity,
        lines: List<SaleLineEntity>,
        packagingLines: List<ReceiptPackagingLine>
    ) {
        val file = generateReceiptPdfFile(context, depotTitle, branchName, depotPhone, sale, lines, packagingLines)
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "application/pdf"
            putExtra(Intent.EXTRA_STREAM, uri)
            putExtra(Intent.EXTRA_SUBJECT, "Reçu DEPOTPRO ${sale.receiptNumber}")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(shareIntent, "Partager le reçu"))
    }

    fun printReceipt(
        context: Context,
        depotTitle: String,
        branchName: String,
        depotPhone: String,
        sale: SaleEntity,
        lines: List<SaleLineEntity>,
        packagingLines: List<ReceiptPackagingLine>
    ) {
        val file = generateReceiptPdfFile(context, depotTitle, branchName, depotPhone, sale, lines, packagingLines)
        val printManager = context.getSystemService(Context.PRINT_SERVICE) as? PrintManager ?: return
        val jobName = "DEPOTPRO_Recu_${sale.receiptNumber}"
        val adapter = object : PrintDocumentAdapter() {
            override fun onLayout(
                oldAttributes: PrintAttributes?,
                newAttributes: PrintAttributes,
                cancellationSignal: CancellationSignal?,
                callback: LayoutResultCallback,
                extras: Bundle?
            ) {
                if (cancellationSignal?.isCanceled == true) {
                    callback.onLayoutCancelled()
                    return
                }
                val info = PrintDocumentInfo.Builder(file.name)
                    .setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT)
                    .setPageCount(1)
                    .build()
                callback.onLayoutFinished(info, true)
            }

            override fun onWrite(
                pages: Array<out PageRange>,
                destination: ParcelFileDescriptor,
                cancellationSignal: CancellationSignal?,
                callback: WriteResultCallback
            ) {
                try {
                    FileInputStream(file).use { input ->
                        FileOutputStream(destination.fileDescriptor).use { output ->
                            input.copyTo(output)
                        }
                    }
                    callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
                } catch (e: Exception) {
                    callback.onWriteFailed(e.message)
                }
            }
        }
        printManager.print(jobName, adapter, PrintAttributes.Builder().build())
    }

    data class ReportProductRow(
        val name: String,
        val unitsSold: Int,
        val revenueFcfa: Long
    )

    fun exportAndShareReport(
        context: Context,
        depotTitle: String,
        branchName: String,
        periodLabel: String,
        salesCount: Int,
        totalCaFcfa: Long,
        totalCollectedFcfa: Long,
        totalCreditsFcfa: Long,
        totalExpensesFcfa: Long,
        estimatedNetFcfa: Long,
        stockValueFcfa: Long,
        lowStockCount: Int,
        customerReceivablesFcfa: Long,
        supplierDebtsFcfa: Long,
        customerCratesDue: Int,
        customerBottlesDue: Int,
        topProducts: List<ReportProductRow>
    ) {
        val dir = File(context.cacheDir, "receipts").apply { mkdirs() }
        val stamp = SimpleDateFormat("yyyyMMdd_HHmm", Locale.US).format(Date())
        val file = File(dir, "Rapport_DEPOTPRO_$stamp.pdf")

        val doc = PdfDocument()
        val pageInfo = PdfDocument.PageInfo.Builder(595, 842, 1).create()
        val page = doc.startPage(pageInfo)
        val canvas = page.canvas

        val titlePaint = Paint().apply {
            textSize = 18f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
            color = android.graphics.Color.parseColor("#0F1E2E")
        }
        val sectionPaint = Paint().apply {
            textSize = 13f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
            color = android.graphics.Color.parseColor("#D98218")
        }
        val bodyPaint = Paint().apply {
            textSize = 11f
            color = android.graphics.Color.parseColor("#1E293B")
        }
        val boldBodyPaint = Paint().apply {
            textSize = 11f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
            color = android.graphics.Color.parseColor("#0F1E2E")
        }
        val mutedPaint = Paint().apply {
            textSize = 10f
            color = android.graphics.Color.parseColor("#64748B")
        }

        val df = SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.FRANCE)
        var y = 48f
        val left = 40f

        canvas.drawText("DEPOTPRO — RAPPORT D'ACTIVITÉ", left, y, titlePaint)
        y += 20f
        canvas.drawText("$depotTitle • $branchName", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Période : $periodLabel • Généré le ${df.format(Date())}", left, y, mutedPaint)
        y += 26f

        canvas.drawText("ACTIVITÉ & TRÉSORERIE", left, y, sectionPaint)
        y += 18f
        canvas.drawText("Chiffre d'affaires ($salesCount ventes) : ${FcfaMath.formatFcfa(totalCaFcfa)}", left, y, boldBodyPaint)
        y += 16f
        canvas.drawText("Encaissements confirmés : ${FcfaMath.formatFcfa(totalCollectedFcfa)}", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Crédits accordés période : ${FcfaMath.formatFcfa(totalCreditsFcfa)}", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Dépenses période : ${FcfaMath.formatFcfa(totalExpensesFcfa)}", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Résultat net indicatif (marge ~14% - dépenses) : ${FcfaMath.formatFcfa(estimatedNetFcfa)}", left, y, boldBodyPaint)
        y += 24f

        canvas.drawText("CRÉANCES, DETTES, STOCK & EMBALLAGES", left, y, sectionPaint)
        y += 18f
        canvas.drawText("Créances clients : ${FcfaMath.formatFcfa(customerReceivablesFcfa)}", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Dettes fournisseurs : ${FcfaMath.formatFcfa(supplierDebtsFcfa)}", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Valeur indicative du stock : ${FcfaMath.formatFcfa(stockValueFcfa)} • Alertes rupture/bas : $lowStockCount", left, y, bodyPaint)
        y += 16f
        canvas.drawText("Emballages dus par clients : $customerCratesDue casier(s) • $customerBottlesDue bouteille(s)", left, y, bodyPaint)
        y += 24f

        canvas.drawText("TOP VENTES PAR PRODUIT", left, y, sectionPaint)
        y += 18f
        if (topProducts.isEmpty()) {
            canvas.drawText("Aucune ligne de vente sur la période sélectionnée.", left, y, mutedPaint)
        } else {
            topProducts.take(12).forEach { row ->
                canvas.drawText("${row.name.take(38)} (${row.unitsSold} u.)", left, y, bodyPaint)
                canvas.drawText(FcfaMath.formatFcfa(row.revenueFcfa), 430f, y, boldBodyPaint)
                y += 16f
            }
        }

        doc.finishPage(page)
        FileOutputStream(file).use { doc.writeTo(it) }
        doc.close()

        val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
        val shareIntent = Intent(Intent.ACTION_SEND).apply {
            type = "application/pdf"
            putExtra(Intent.EXTRA_STREAM, uri)
            putExtra(Intent.EXTRA_SUBJECT, "Rapport DEPOTPRO — $periodLabel")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(shareIntent, "Exporter ou partager le rapport"))
    }
}
