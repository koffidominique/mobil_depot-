package com.depotpro.mobile

import android.content.Context
import android.os.Bundle
import android.os.CancellationSignal
import android.os.ParcelFileDescriptor
import android.print.PageRange
import android.print.PrintAttributes
import android.print.PrintDocumentAdapter
import android.print.PrintDocumentInfo
import android.print.PrintManager
import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactMethod
import java.io.File
import java.io.FileOutputStream

/**
 * TurboModule d'impression : transmet le PDF généré par le moteur au service
 * d'impression du système. Aucun calcul financier ici.
 */
class ReceiptPrinterModule(reactContext: ReactApplicationContext) : NativeReceiptPrinterSpec(reactContext) {
  @ReactMethod
  override fun printPdf(filePath: String, promise: Promise) {
    val file = File(filePath)
    if (!file.isFile) {
      promise.reject("PDF_NOT_FOUND", "Fichier PDF introuvable.")
      return
    }
    val printManager = reactApplicationContext.getSystemService(Context.PRINT_SERVICE) as? PrintManager
    if (printManager == null) {
      promise.reject("PRINT_UNAVAILABLE", "Service d'impression indisponible.")
      return
    }
    val adapter = object : PrintDocumentAdapter() {
      override fun onLayout(
        oldAttributes: PrintAttributes?,
        newAttributes: PrintAttributes?,
        cancellationSignal: CancellationSignal?,
        callback: LayoutCallback?,
        extras: Bundle?,
      ) {
        if (callback == null) return
        val info = PrintDocumentInfo.Builder("depotpro-recu.pdf")
          .setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT)
          .setPageCount(1)
          .build()
        callback.onLayoutFinished(info, true)
      }

      override fun onWrite(
        pages: Array<out PageRange>?,
        destination: ParcelFileDescriptor?,
        cancellationSignal: CancellationSignal?,
        callback: WriteCallback?,
        extras: Bundle?,
      ) {
        if (destination == null || callback == null) return
        try {
          FileOutputStream(destination.fileDescriptor).use { output -> output.write(file.readBytes()) }
          callback.onWriteFinished(arrayOf(PageRange.ALL_PAGES))
        } catch (error: Exception) {
          callback.onWriteFailed(error.message)
        }
      }
    }
    try {
      printManager.print("DEPOTPRO-Reçu", adapter, null)
      promise.resolve(true)
    } catch (error: Exception) {
      promise.reject("PRINT_FAILED", error.message, error)
    }
  }
}
