import Foundation

/// Couche iOS typée au-dessus de l'ABI C du moteur Rust.
/// Toutes les décisions métier restent dans `depot_execute`.
final class DepotCoreSwift {
  static let shared = DepotCoreSwift()

  private let queue = DispatchQueue(label: "com.depotpro.mobile.swift-core")

  enum CoreFailure: Error {
    case invalidRequest
    case invalidResponse
    case core(code: String, message: String)
  }

  /// Exécute une commande hors du thread principal, puis revient sur le thread principal.
  func execute(
    command: String,
    payload: [String: Any],
    completion: @escaping (Result<[String: Any], Error>) -> Void
  ) {
    guard let requestData = try? JSONSerialization.data(withJSONObject: ["command": command, "payload": payload]),
          let request = String(data: requestData, encoding: .utf8) else {
      completion(.failure(CoreFailure.invalidRequest))
      return
    }

    queue.async {
      let raw = request.withCString { depot_execute($0) }
      defer {
        if let raw { depot_free(raw) }
      }
      guard let raw,
            let response = String(cString: raw),
            let data = response.data(using: .utf8),
            let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        DispatchQueue.main.async { completion(.failure(CoreFailure.invalidResponse)) }
        return
      }
      let result: Result<[String: Any], Error>
      if object["ok"] as? Bool == true {
        result = .success((object["data"] as? [String: Any]) ?? [:])
      } else {
        let error = (object["error"] as? [String: Any]) ?? [:]
        result = .failure(CoreFailure.core(
          code: error["code"] as? String ?? "UNKNOWN_CORE_ERROR",
          message: error["message"] as? String ?? "Erreur du moteur Rust."
        ))
      }
      DispatchQueue.main.async { completion(result) }
    }
  }
}
