# Compatibilité des API publiques

Ce document satisfait la règle 24 : tout changement d'API est explicité. Aucune commande existante n'a été supprimée (règle 2).

| Commande / interface | Changement | Compatibilité |
| --- | --- | --- |
| `initializeDatabase` (FFI) | Nouveau champ optionnel `seed_demo` (défaut `true` dans le moteur). Réponse ajoute `seeded_demo`. | Anciens appels inchangés. Le mobile natif passe désormais `false` ; l'aperçu web passe `true`. |
| `initializeDatabase(path, seedDemo?)` (client TS) | Nouveau paramètre optionnel. | Appel historique à un seul argument toujours valide. |
| `backupDatabase(passphrase?, destinationPath?)` (client TS) | **Changement de signature** : le premier paramètre est désormais la phrase, le second le chemin. | Appels historiques `backupDatabase(path)` doivent devenir `backupDatabase(undefined, path)`. FFI : champs optionnels `passphrase` et `destination_path`. |
| `restoreDatabase(sourcePath, passphrase?)` (client TS) | Nouveau paramètre optionnel. | Ancien appel toujours valide pour une sauvegarde non chiffrée ; une enveloppe `.dpbkp` exige la phrase. |
| `changePin(oldPin, newPin)` (FFI/client) | Nouvelle commande. | Ajout purement additif. |
| Schéma local | Migration 10 : `users.pin_salt`, `users.pin_kdf`, version 10. | Additive ; les hashes SHA-256 existants restent vérifiables puis convertis au prochain succès. |
| `/api/v1/sync/*` | Aucun changement de contrat dans ce lot. | Voir `docs/API_SYNC_V1.md`. |
