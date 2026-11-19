package com.depotpro.mobile

import com.facebook.react.bridge.Promise
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReactMethod
import java.util.concurrent.Executors

/**
 * TurboModule React Native (New Architecture) délégant toutes les décisions métier
 * au moteur Rust. Aucune règle commerciale n'est réimplémentée ici.
 */
class DepotCoreModule(reactContext: ReactApplicationContext) : NativeDepotCoreSpec(reactContext) {
  companion object {
    private var libraryLoaded = false

    init {
      try {
        System.loadLibrary("depotpro_mobile_core")
        libraryLoaded = true
      } catch (_: UnsatisfiedLinkError) {
        libraryLoaded = false
      }
    }
  }

  private val executor = Executors.newSingleThreadExecutor { runnable ->
    Thread(runnable, "depotpro-rust-core").apply { isDaemon = true }
  }

  private external fun nativeExecute(request: String): String

  @ReactMethod
  override fun executeCommand(request: String, promise: Promise) {
    if (!libraryLoaded) {
      promise.reject(
        "RUST_LIBRARY_MISSING",
        "Bibliothèque Rust absente : exécuter npm run rust:android avant de compiler l'APK.",
      )
      return
    }
    executor.execute {
      try {
        promise.resolve(nativeExecute(request))
      } catch (error: Throwable) {
        promise.reject("RUST_CORE_ERROR", error.message, error)
      }
    }
  }

  override fun invalidate() {
    executor.shutdownNow()
    super.invalidate()
  }
}
